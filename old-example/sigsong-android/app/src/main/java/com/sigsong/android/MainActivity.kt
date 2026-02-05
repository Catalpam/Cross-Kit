package com.sigsong.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.Surface
import androidx.compose.material3.MaterialTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            SigSongTheme {
                Surface(color = MaterialTheme.colorScheme.background) {
                    SigSongApp()
                }
            }
        }
    }
}
