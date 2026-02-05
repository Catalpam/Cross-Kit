pluginManagement {
    repositories {
        gradlePluginPortal()
        google()
        maven { url = uri("https://maven.google.com") }
        mavenCentral()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "SigSongAndroid"
include(":app")
