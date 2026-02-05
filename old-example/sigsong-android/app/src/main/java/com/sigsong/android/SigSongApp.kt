package com.sigsong.android

import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.sigsong.sdk.uniffi.sig_song_sdk.ClFeedRecommands
import com.sigsong.sdk.uniffi.sig_song_sdk.ClSongBrief
import com.sigsong.sdk.uniffi.sig_song_sdk.ClSongInfo
import com.sigsong.sdk.uniffi.sig_song_sdk.ClSongLyric
import com.sigsong.sdk.uniffi.sig_song_sdk.ClUserInfo
import com.sigsong.sdk.uniffi.sig_song_sdk.ClWordInfo
import com.sigsong.sdk.uniffi.sig_song_sdk.InvokeFfi
import com.sigsong.sdk.uniffi.sig_song_sdk.InvokeManager
import com.sigsong.sdk.uniffi.sig_song_sdk.ToastType
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private data class ToastMessage(val type: ToastType, val message: String)

private class AndroidInvokeFfi(
    private val appContext: Context,
    private val toaster: MutableSharedFlow<ToastMessage>,
) : InvokeFfi {
    override fun getDocumentPath(): String {
        val baseDir = File(appContext.filesDir, "sigsong")
        if (!baseDir.exists()) {
            baseDir.mkdirs()
        }
        return baseDir.absolutePath
    }

    override fun showToast(toastType: ToastType, message: String) {
        toaster.tryEmit(ToastMessage(toastType, message))
    }
}

@Composable
fun SigSongApp() {
    val context = LocalContext.current
    val snackbarHostState = remember { SnackbarHostState() }
    val coroutineScope = rememberCoroutineScope()
    val toastFlow = remember { MutableSharedFlow<ToastMessage>(extraBufferCapacity = 4) }

    var initTrigger by remember { mutableStateOf(0) }
    var invokeManager by remember { mutableStateOf<InvokeManager?>(null) }
    var currentUser by remember { mutableStateOf<ClUserInfo?>(null) }
    var selectedSong by remember { mutableStateOf<ClSongInfo?>(null) }
    var isLoading by remember { mutableStateOf(true) }
    var initError by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        toastFlow.collect { toast ->
            snackbarHostState.showSnackbar(toast.message)
        }
    }

    LaunchedEffect(initTrigger) {
        isLoading = true
        initError = null
        invokeManager?.close()
        invokeManager = null
        currentUser = null
        withContext(Dispatchers.IO) {
            runCatching {
                val ffi = AndroidInvokeFfi(context.applicationContext, toastFlow)
                InvokeManager(ffi)
            }.onSuccess { manager ->
                invokeManager = manager
                currentUser = runCatching { manager.currentUser() }.getOrNull()
            }.onFailure { throwable ->
                initError = throwable.message ?: throwable.toString()
            }
        }
        isLoading = false
    }

    Scaffold(snackbarHost = { SnackbarHost(snackbarHostState) }) { padding ->
        when {
            isLoading -> LoadingScreen(modifier = Modifier
                .padding(padding)
                .fillMaxSize())

            initError != null -> ErrorScreen(
                message = initError!!,
                onRetry = { initTrigger += 1 },
                modifier = Modifier
                    .padding(padding)
                    .fillMaxSize()
            )

            invokeManager == null -> LoadingScreen(modifier = Modifier
                .padding(padding)
                .fillMaxSize())

            currentUser == null -> LoginScreen(
                manager = invokeManager!!,
                snackbarHostState = snackbarHostState,
                onLoggedIn = { user ->
                    currentUser = user
                },
                modifier = Modifier
                    .padding(padding)
                    .fillMaxSize()
            )

            selectedSong != null -> SongDetailScreen(
                song = selectedSong!!,
                onBack = { selectedSong = null },
                onFetchWordInfo = { surface ->
                    coroutineScope.launch {
                        val infoResult = runCatching {
                            withContext(Dispatchers.IO) {
                                invokeManager!!.getWordInfo(surface)
                            }
                        }
                        infoResult.onSuccess { infos ->
                            val summary = infos.firstOrNull()?.let { formatWordInfoSummary(it) }
                                ?: "未找到相关释义"
                            snackbarHostState.showSnackbar(summary)
                        }.onFailure { throwable ->
                            snackbarHostState.showSnackbar(throwable.message ?: "加载词典失败")
                        }
                    }
                },
                modifier = Modifier
                    .padding(padding)
                    .fillMaxSize()
            )

            else -> HomeScreen(
                manager = invokeManager!!,
                user = currentUser!!,
                snackbarHostState = snackbarHostState,
                onLogout = {
                    coroutineScope.launch {
                        runCatching { withContext(Dispatchers.IO) { invokeManager!!.logout() } }
                        currentUser = null
                        selectedSong = null
                    }
                },
                onSongSelected = { info -> selectedSong = info },
                modifier = Modifier
                    .padding(padding)
                    .fillMaxSize()
            )
        }
    }
}

@Composable
private fun LoadingScreen(modifier: Modifier = Modifier) {
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        CircularProgressIndicator()
        Spacer(modifier = Modifier.height(16.dp))
        Text("正在加载…")
    }
}

@Composable
private fun ErrorScreen(message: String, onRetry: () -> Unit, modifier: Modifier = Modifier) {
    Column(
        modifier = modifier.padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(text = "初始化失败", style = MaterialTheme.typography.titleMedium)
        Spacer(modifier = Modifier.height(8.dp))
        Text(text = message, style = MaterialTheme.typography.bodyMedium)
        Spacer(modifier = Modifier.height(16.dp))
        Button(onClick = onRetry) {
            Text("重试")
        }
    }
}

@Composable
private fun LoginScreen(
    manager: InvokeManager,
    snackbarHostState: SnackbarHostState,
    onLoggedIn: (ClUserInfo) -> Unit,
    modifier: Modifier = Modifier
) {
    var username by rememberSaveable { mutableStateOf("demo@sigsong") }
    var password by rememberSaveable { mutableStateOf("sigsong123") }
    var isSubmitting by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()

    Column(
        modifier = modifier
            .padding(24.dp),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Text(text = "欢迎来到 SigSong", style = MaterialTheme.typography.headlineSmall)
        Spacer(modifier = Modifier.height(24.dp))
        OutlinedTextField(
            value = username,
            onValueChange = { username = it },
            label = { Text("账号") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth()
        )
        Spacer(modifier = Modifier.height(12.dp))
        OutlinedTextField(
            value = password,
            onValueChange = { password = it },
            label = { Text("密码") },
            singleLine = true,
            visualTransformation = PasswordVisualTransformation(),
            modifier = Modifier.fillMaxWidth()
        )
        Spacer(modifier = Modifier.height(24.dp))
        Button(
            onClick = {
                if (username.isBlank() || password.isBlank()) {
                    return@Button
                }
                isSubmitting = true
                scope.launch {
                    val result = runCatching {
                        withContext(Dispatchers.IO) {
                            manager.login(username, password)
                        }
                    }
                    result.onSuccess { user ->
                        onLoggedIn(user)
                    }.onFailure { throwable ->
                        snackbarHostState.showSnackbar(throwable.message ?: "登录失败")
                    }
                    isSubmitting = false
                }
            },
            enabled = !isSubmitting,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text(if (isSubmitting) "登录中…" else "登录")
        }
    }
}

@Composable
private fun HomeScreen(
    manager: InvokeManager,
    user: ClUserInfo,
    snackbarHostState: SnackbarHostState,
    onLogout: () -> Unit,
    onSongSelected: (ClSongInfo) -> Unit,
    modifier: Modifier = Modifier
) {
    val scope = rememberCoroutineScope()

    var recommendations by remember { mutableStateOf<ClFeedRecommands?>(null) }
    var recLoading by remember { mutableStateOf(false) }
    var recError by remember { mutableStateOf<String?>(null) }
    var searchKeyword by rememberSaveable { mutableStateOf("") }
    var searchResults by remember { mutableStateOf<List<ClSongBrief>>(emptyList()) }
    var searchLoading by remember { mutableStateOf(false) }
    var searchError by remember { mutableStateOf<String?>(null) }

    fun loadRecommendations(lastId: Int? = null) {
        recLoading = true
        recError = null
        scope.launch {
            val result = runCatching {
                withContext(Dispatchers.IO) {
                    manager.getFeedRecommendSongs(lastId)
                }
            }
            result.onSuccess { feed ->
                recommendations = feed
            }.onFailure { throwable ->
                recError = throwable.message ?: "加载推荐失败"
            }
            recLoading = false
        }
    }

    LaunchedEffect(Unit) {
        loadRecommendations(null)
    }

    fun fetchSong(brief: ClSongBrief) {
        scope.launch {
            val result = runCatching {
                withContext(Dispatchers.IO) { manager.getSongById(brief.id) }
            }
            result.onSuccess { info ->
                onSongSelected(info)
            }.onFailure { throwable ->
                snackbarHostState.showSnackbar(throwable.message ?: "加载歌曲失败")
            }
        }
    }

    Column(
        modifier = modifier
            .padding(16.dp)
    ) {
        Text(text = "你好，${user.name}", style = MaterialTheme.typography.titleMedium)
        Spacer(modifier = Modifier.height(12.dp))
        OutlinedTextField(
            value = searchKeyword,
            onValueChange = { searchKeyword = it },
            label = { Text("搜索歌曲") },
            singleLine = true,
            modifier = Modifier.fillMaxWidth()
        )
        Spacer(modifier = Modifier.height(8.dp))
        Button(
            onClick = {
                if (searchKeyword.isBlank()) return@Button
                searchLoading = true
                searchError = null
                scope.launch {
                    val result = runCatching {
                        withContext(Dispatchers.IO) {
                            manager.searchSong(searchKeyword)
                        }
                    }
                    result.onSuccess { songs ->
                        searchResults = songs
                    }.onFailure { throwable ->
                        searchError = throwable.message ?: "搜索失败"
                    }
                    searchLoading = false
                }
            },
            enabled = !searchLoading,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text(if (searchLoading) "搜索中…" else "搜索")
        }

        if (searchError != null) {
            Text(searchError!!, color = MaterialTheme.colorScheme.error)
        }

        if (searchResults.isNotEmpty()) {
            Text(
                text = "搜索结果",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(top = 16.dp, bottom = 8.dp)
            )
            LazyColumn {
                items(searchResults) { brief ->
                    SongRow(brief = brief, onClick = { fetchSong(brief) })
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = "今日推荐",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f)
            )
            OutlinedButton(onClick = { loadRecommendations(recommendations?.recommandId) }) {
                Text("下一组")
            }
            TextButton(onClick = onLogout) {
                Text("退出")
            }
        }

        when {
            recLoading -> {
                CircularProgressIndicator(modifier = Modifier.padding(16.dp))
            }
            recError != null -> {
                Column {
                    Text(recError!!, color = MaterialTheme.colorScheme.error)
                    TextButton(onClick = { loadRecommendations(recommendations?.recommandId) }) {
                        Text("重试")
                    }
                }
            }
            recommendations != null -> {
                LazyColumn {
                    items(recommendations!!.songBriefs) { brief ->
                        SongRow(brief = brief, onClick = { fetchSong(brief) })
                    }
                }
            }
        }
    }
}

@Composable
private fun SongRow(brief: ClSongBrief, onClick: () -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp)
            .background(MaterialTheme.colorScheme.surfaceVariant, shape = MaterialTheme.shapes.medium)
            .padding(16.dp)
    ) {
        Text(brief.title, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Text("ID: ${brief.id}", style = MaterialTheme.typography.bodySmall)
        Spacer(modifier = Modifier.height(8.dp))
        Button(onClick = onClick, modifier = Modifier.align(Alignment.End)) {
            Text("查看详情")
        }
    }
}

@Composable
private fun SongDetailScreen(
    song: ClSongInfo,
    onBack: () -> Unit,
    onFetchWordInfo: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(
        modifier = modifier.padding(16.dp)
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            TextButton(onClick = onBack) {
                Text("返回")
            }
            Spacer(modifier = Modifier.weight(1f))
            Text(text = song.title, style = MaterialTheme.typography.titleLarge)
        }
        Spacer(modifier = Modifier.height(12.dp))
        LazyColumn {
            items(song.lyrics) { lyric ->
                LyricCard(lyric = lyric, onFetchWordInfo = onFetchWordInfo)
            }
        }
    }
}

@Composable
private fun LyricCard(lyric: ClSongLyric, onFetchWordInfo: (String) -> Unit) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = 8.dp)
            .background(MaterialTheme.colorScheme.secondaryContainer, shape = MaterialTheme.shapes.medium)
            .padding(16.dp)
    ) {
        Text(lyric.text, style = MaterialTheme.typography.bodyLarge)
        if (lyric.zhCn.isNotBlank()) {
            Spacer(modifier = Modifier.height(4.dp))
            Text(lyric.zhCn, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSecondaryContainer)
        }
        if (lyric.elements.isNotEmpty()) {
            TextButton(onClick = { onFetchWordInfo(lyric.elements.first().surface) }) {
                Text("查看 “${lyric.elements.first().surface}” 的词典")
            }
        }
    }
}

private fun formatWordInfoSummary(info: ClWordInfo): String {
    val tone = if (info.tone.isNotEmpty()) info.tone.joinToString(",") else ""
    val definition = info.senses.firstOrNull()?.meanings?.firstOrNull()?.definition ?: ""
    return buildString {
        append(info.word)
        if (info.pronounce.isNotBlank()) {
            append(" · ")
            append(info.pronounce)
        }
        if (tone.isNotBlank()) {
            append(" · ")
            append("调: $tone")
        }
        if (definition.isNotBlank()) {
            append(" · ")
            append(definition)
        }
    }
}

// Saver for ClSongInfo to keep state across configuration changes
