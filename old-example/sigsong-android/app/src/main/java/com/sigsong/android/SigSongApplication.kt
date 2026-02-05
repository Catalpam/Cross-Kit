package com.sigsong.android

import android.app.Application

class SigSongApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        System.loadLibrary("sig_song_sdk")
    }
}
