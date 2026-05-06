package com.example.crosskit_example_android

import java.io.File
import org.junit.Assert.assertTrue
import org.junit.Test

class ExampleUnitTest {
    @Test
    fun generatedDependencyCoordinateStaysInAppBuildFile() {
        val buildFile = projectFile("app/build.gradle.kts")
        assertTrue(buildFile.readText().contains("implementation(\"$GENERATED_DEPENDENCY\")"))
    }

    @Test
    fun localMavenRepositoryPointsAtPackagedExampleOutput() {
        val settings = projectFile("settings.gradle.kts")
        assertTrue(settings.readText().contains("url = uri(\"../dist/android/maven\")"))
    }

    private companion object {
        const val GENERATED_DEPENDENCY = "com.crosskit:crosskitshoppingcartshared:0.1.0"

        fun projectFile(path: String): File {
            val fromRoot = File(path)
            if (fromRoot.exists()) {
                return fromRoot
            }
            return File("..", path)
        }
    }
}
