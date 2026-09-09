// SPDX-License-Identifier: AGPL-3.0-or-later
pluginManagement { repositories { google(); mavenCentral(); gradlePluginPortal() } }
dependencyResolutionManagement { repositories { google(); mavenCentral() } }
rootProject.name = "irondoc-reader"
include(":app")
