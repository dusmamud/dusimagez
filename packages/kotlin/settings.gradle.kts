// dusimagez — Kotlin (Android) bindings, Gradle settings.
// Library module: :library  (package com.dusimagez, AAR via Gradle).
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "dusimagez"
include(":library")
