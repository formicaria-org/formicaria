#!/bin/sh
# Inject a **foreground service** into the generated Android project so Android keeps formicaria's
# process — and its network — alive while the study assistant downloads its model and answers
# questions. Without it, Doze/App-Standby throttle the backgrounded app's network (the ~150 MB model
# download stalls with DNS failures — seen on-device) and LMKD is free to reap the model subprocess
# mid-generation.
#
# gen/android is generated (and gitignored), so — like ci/android-stage-runtime.sh — this committed
# script re-applies the edits idempotently. `MainActivity.kt` lives outside `generated/`, so tauri does
# not overwrite it; we own it. Run before an android build (android-apk/android-release depend on it).
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
pkg_dir="$root/mobile/src-tauri/gen/android/app/src/main/java/dev/formicaria/notes"
manifest="$root/mobile/src-tauri/gen/android/app/src/main/AndroidManifest.xml"
[ -d "$pkg_dir" ] || { echo "android-inject-service: $pkg_dir missing — run 'tauri android init' first" >&2; exit 1; }
[ -f "$manifest" ] || { echo "android-inject-service: $manifest missing" >&2; exit 1; }

# 1) The service itself. Overwrite each run (idempotent by construction).
cp "$root/mobile/android/kotlin/AgentService.kt" "$pkg_dir/AgentService.kt"

# 1b) No reminders on Android (`decisions.md#track-m`, 2026-10-31, *the phone sets no alarms and
#     starts at no boot*). `Reminders.kt` — wake-up alarms, a start-at-boot receiver, notifications
#     the page wrote — came with 0.7.7, when Play Protect began to call the app harmful. A copy an
#     earlier run left beside `MainActivity.kt` is removed, or Gradle would still compile it.
rm -f "$pkg_dir/Reminders.kt"

# 2) MainActivity starts the service on launch. Overwrite the whole file (small, stable, and tauri
#    does not regenerate it — it is outside generated/).
cp "$root/mobile/android/kotlin/MainActivity.kt" "$pkg_dir/MainActivity.kt"

# 2a) **The phone's page protections** (`decisions.md` 2026-11-17, *privacy switches*): what the
#     Privacy sheet's switches do in the page views. Plain functions over URLs, plus the scripts a page
#     gets at its start; `MainActivity` calls them. Ad and tracker blocking lives in `Blocker`.
cp "$root/mobile/android/kotlin/PagePrivacy.kt" "$pkg_dir/PagePrivacy.kt"


# 2a') **Save media on the phone** (`decisions.md` 2026-11-17): yt-dlp and Python inside the
#      app (youtubedl-android, GPL-3.0: the phone APK is distributed under GPL-3.0 because of it; the
#      source stays MIT). The shell runs it; the core says what to ask and files the result.
cp "$root/mobile/android/kotlin/MediaRun.kt" "$pkg_dir/MediaRun.kt"

# 2a'') **Music plays on in the background** (`decisions.md` 2026-11-17; `music-plan-2026-11-13.md`
#       M4(b)): a Media3 player in a media service, holding the whole queue, so the lock screen, the
#       notification, headphones, Bluetooth and the car control it, and it goes on with the screen
#       off. The app's player (`player.svelte.ts`) is its remote and mirrors what it says.
cp "$root/mobile/android/kotlin/PlaybackService.kt" "$pkg_dir/PlaybackService.kt"

# 2d) **Trackers in other apps refused, phone-wide** (`decisions.md` 2026-11-17): a local VPN that
#     handles only address lookups (DNS). A lookup of a tracker's address gets "no such address";
#     every other is passed to the network's own resolver. No traffic but lookups enters it, and
#     nothing leaves the phone that would not have. Which app asked for which tracker is counted.
cp "$root/mobile/android/kotlin/TrackerVpn.kt" "$pkg_dir/TrackerVpn.kt"

# 2b) **Do NOT try to patch anything under `generated/` from here — it will not survive.**
#
# Attempted 2026-07-31, for a good reason: `RustWebViewClient` overrides no `onRenderProcessGone`,
# and the framework default for that is to **kill the app process** — so on a memory-tight phone
# (a phone that swaps ~3 GB while LMKD reaps Chrome and the Play Store, formicaria surviving on this
# very service while holding ~2 GB of model mappings) the app can simply vanish with nothing
# anywhere saying why. A logging override would at least name the cause, since `didCrash()`
# distinguishes "the system took the renderer" from "the renderer crashed".
#
# **It was applied, and then silently overwritten**: `tauri android build` re-runs its own codegen
# over `generated/` *during* the build, i.e. after this pre-build hook has finished. The patch
# reappeared and vanished on every build and no error was raised anywhere — verified by grepping the
# file mid-build. There is no post-codegen, pre-compile hook to hang it on. `AgentService.kt` and
# `MainActivity.kt` are only editable here because they sit **outside** `generated/`.
#
# So the diagnostic is not built, and the gap is recorded in `docs/context/known-issues.md` instead
# of being pretended to. If it is ever worth having, it needs a Gradle source-transform (or an
# upstream tauri hook), not another awk block here.

# 3) Manifest: the foreground-service permissions (after INTERNET) and the <service> (before
#    </application>). Idempotent — only inject what is absent.
if ! grep -q 'FOREGROUND_SERVICE"' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /android.permission.INTERNET/ && !p {
            print
            print "    <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE\" />"
            print "    <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE_DATA_SYNC\" />"
            print "    <uses-permission android:name=\"android.permission.POST_NOTIFICATIONS\" />"
            p=1; next
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
# Microphone: in-app audio recording is a plain getUserMedia in the WebView, and wry's
# onPermissionRequest already asks for RECORD_AUDIO + MODIFY_AUDIO_SETTINGS at runtime and grants the
# WebView's AUDIO_CAPTURE on approval — but Android only lets it *request* a permission that is
# declared here. Declaring these makes in-app recording work on the phone with the same web code the
# browser build uses (any-device by construction). Idempotent.
if ! grep -q 'RECORD_AUDIO' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /android.permission.INTERNET/ && !a {
            print
            print "    <uses-permission android:name=\"android.permission.RECORD_AUDIO\" />"
            print "    <uses-permission android:name=\"android.permission.MODIFY_AUDIO_SETTINGS\" />"
            a=1; next
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
# No update installer (`decisions.md#track-m`, 2026-10-31, *a phone update opens in the browser*):
# `REQUEST_INSTALL_PACKAGES` is not declared, because Update now opens the newest APK in the
# browser and Android's own installer takes it from there. A manifest written by an earlier run of
# this script still carries the line, so it is taken out here.
sed -i '/android.permission.REQUEST_INSTALL_PACKAGES/d' "$manifest"
# **The camera must be visible to the app, or "take a photo" opens the gallery** (owner, 2026-10-18:
# food pictures could only come from the gallery). wry opens the camera only after asking Android
# whether any app answers `ACTION_IMAGE_CAPTURE` (`RustWebChromeClient.kt`, `resolveActivity`), and
# since Android 11 an app that targets API 30+ sees only the apps it declares it will talk to — so
# the answer was "none", and wry fell back to the file picker without a word. This `<queries>`
# block is that declaration, for photos and videos; it grants nothing and asks for no permission.
# It sits beside `<application>`, as the manifest requires. Idempotent.
if ! grep -q 'IMAGE_CAPTURE' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<application/ && !q {
            print "    <queries>"
            print "        <intent>"
            print "            <action android:name=\"android.media.action.IMAGE_CAPTURE\" />"
            print "        </intent>"
            print "        <intent>"
            print "            <action android:name=\"android.media.action.VIDEO_CAPTURE\" />"
            print "        </intent>"
            print "    </queries>"
            q=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi
if ! grep -q 'AgentService' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/application>/ && !s {
            print "        <service"
            print "            android:name=\".AgentService\""
            print "            android:exported=\"false\""
            print "            android:foregroundServiceType=\"dataSync\" />"
            s=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# No reminders on Android (2026-10-31, see 1b): no `RECEIVE_BOOT_COMPLETED` and no receivers. A
# manifest written by an earlier run still carries them, so they are taken out here.
sed -i '/android.permission.RECEIVE_BOOT_COMPLETED/d; /<receiver android:name=".ReminderReceiver"/d' "$manifest"
sed -i '/<receiver android:name=".BootReceiver"/,/<\/receiver>/d' "$manifest"

# Share: formicaria in Android's share menu for pictures (`decisions.md#ui`, 2026-10-20). One file
# or several; the activity takes them (`takeShared`). Pictures only: what a screenshot is, and what
# the note editor draws. Idempotent.
if ! grep -q 'android.intent.action.SEND"' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/activity>/ && !sh {
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.SEND\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"image/*\" />"
            print "            </intent-filter>"
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.SEND_MULTIPLE\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"image/*\" />"
            print "            </intent-filter>"
            sh=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# Bring in (2026-11-16): formicaria in Share for text (a page's link, a passage), and in **the
# text-selection menu of every app** (`PROCESS_TEXT`, labelled with the app's name). Separate from the
# pictures' block, so a manifest written before still gets it. Idempotent.
if ! grep -q 'android.intent.action.PROCESS_TEXT' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/activity>/ && !tx {
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.SEND\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"text/plain\" />"
            print "            </intent-filter>"
            print "            <intent-filter>"
            print "                <action android:name=\"android.intent.action.PROCESS_TEXT\" />"
            print "                <category android:name=\"android.intent.category.DEFAULT\" />"
            print "                <data android:mimeType=\"text/plain\" />"
            print "            </intent-filter>"
            tx=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# **A watch's activity files** (`decisions.md` 2026-11-20, *The phone takes watch files*): formicaria
# in Share for a `.fit`. Android knows no type for one, so the app sharing it says a generic one;
# `MainActivity.takeShared` takes it only when its name ends `.fit` and its header carries the FIT
# signature, and refuses anything else with a sentence. Not `*/*`: formicaria would then be offered
# for every file on the phone. Its own block, so a manifest written before still gets it. Idempotent.
if ! grep -q 'application/vnd.ant.fit' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/activity>/ && !ft {
            for (a = 0; a < 2; a++) {
                print "            <intent-filter>"
                print "                <action android:name=\"android.intent.action." (a ? "SEND_MULTIPLE" : "SEND") "\" />"
                print "                <category android:name=\"android.intent.category.DEFAULT\" />"
                print "                <data android:mimeType=\"application/octet-stream\" />"
                print "                <data android:mimeType=\"application/vnd.ant.fit\" />"
                print "                <data android:mimeType=\"application/fit\" />"
                print "            </intent-filter>"
            }
            ft=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# **Save media's tools** (2026-11-17): youtubedl-android (yt-dlp and Python), in the generated Gradle file
# (regenerated by `tauri android init`, so patched here, idempotently). Only the architecture being
# built (`abiList`, which Tauri passes; arm64 by default): the tools' native files are large.
gradle="$root/mobile/src-tauri/gen/android/app/build.gradle.kts"
if [ -f "$gradle" ] && ! grep -q 'youtubedl-android' "$gradle"; then
    tmpf=$(mktemp)
    awk '
        /^dependencies \{/ && !d { print; print "    implementation(\"io.github.junkfood02.youtubedl-android:library:0.18.1\")"; d=1; next }
        /defaultConfig \{/ && !n { print; print "        ndk { abiFilters += ((project.findProperty(\"abiList\") as String?) ?: \"arm64-v8a\").split(\",\") }"; n=1; next }
        { print }
    ' "$gradle" > "$tmpf" && mv "$tmpf" "$gradle"
fi

# **Save media's tools kept whole by R8** (2026-11-18). The release build shrinks and renames classes;
# youtubedl-android unpacks Python and yt-dlp with Apache Commons Compress, which finds its zip helpers
# by class name, updates yt-dlp through Apache Commons IO (shrunk: a class loader of null), and reads
# JSON with Jackson, which finds classes by name too. Shrunk, the first Save media failed on
# every phone with *"class i3.a is not a concrete class"* (0.54.0), while the debug build, never
# shrunk, passed every check. `phone-media-check` now runs the release build. Picked up by the Gradle
# file's `fileTree("**/*.pro")`.
if [ -f "$gradle" ]; then
    cat > "$(dirname "$gradle")/proguard-media.pro" <<'PRO'
# Written by ci/android-inject-service.sh: Save media's tools look classes up by name.
-keep class com.yausername.** { *; }
-keep class org.apache.commons.compress.** { *; }
-dontwarn org.apache.commons.compress.**
-keep class org.apache.commons.io.** { *; }
-dontwarn org.apache.commons.io.**
-keep class com.fasterxml.jackson.** { *; }
-dontwarn com.fasterxml.jackson.**
-keepattributes *Annotation*,Signature,InnerClasses,EnclosingMethod
PRO
fi

# **No ffmpeg in the app** (2026-11-17: 35 MB; the core makes MP3 and writes tags, Android joins a
# video's streams): a Gradle file patched before loses the line.
if [ -f "$gradle" ]; then
    sed -i '/youtubedl-android:ffmpeg/d' "$gradle"
fi

# The native player's library (Media3), its own step so a Gradle file patched before still gets it.
# 1.6.1: the newest whose Kotlin metadata this project's Kotlin (1.9) reads; 1.11 needs Kotlin 2.2.
if [ -f "$gradle" ] && ! grep -q 'media3-session' "$gradle"; then
    tmpf=$(mktemp)
    awk '
        /^dependencies \{/ && !d { print; print "    implementation(\"androidx.media3:media3-exoplayer:1.6.1\")"; print "    implementation(\"androidx.media3:media3-session:1.6.1\")"; d=1; next }
        { print }
    ' "$gradle" > "$tmpf" && mv "$tmpf" "$gradle"
fi

# **The tracker block's service** (2026-11-17, in the one APK for everyone): Android binds it only
# with the VPN permission the system holds; it asks the person before the first start. Idempotent.
if ! grep -q 'TrackerVpn' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<\/application>/ && !v {
            print "        <service android:name=\".TrackerVpn\" android:permission=\"android.permission.BIND_VPN_SERVICE\" android:exported=\"false\">"
            print "            <intent-filter><action android:name=\"android.net.VpnService\" /></intent-filter>"
            print "        </service>"
            v=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# **The media service** (background playback, 2026-11-17): declared with its type and permission
# (`ci/checks.sh` allows exactly this one beyond the earlier list). Idempotent.
if ! grep -q 'PlaybackService' "$manifest"; then
    tmpf=$(mktemp)
    awk '
        /<application/ && !p { print "    <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE_MEDIA_PLAYBACK\" />"; p=1 }
        /<\/application>/ && !s {
            print "        <service android:name=\".PlaybackService\" android:foregroundServiceType=\"mediaPlayback\" android:exported=\"true\">"
            print "            <intent-filter><action android:name=\"androidx.media3.session.MediaSessionService\" /></intent-filter>"
            print "        </service>"
            s=1
        }
        { print }
    ' "$manifest" > "$tmpf" && mv "$tmpf" "$manifest"
fi

# **Open with…** (2026-11-17): a content link to a vault file, for another app to play, through the
# FileProvider Tauri's template declares (`${applicationId}.fileprovider`): its paths gain the app's
# own storage (`files-path`). A grant is per file, per opening. Idempotent.
paths_xml="$(dirname "$manifest")/res/xml/file_paths.xml"
rm -f "$(dirname "$manifest")/res/xml/fm_files.xml"
if [ -f "$paths_xml" ] && ! grep -q 'files-path' "$paths_xml"; then
    sed -i 's#</paths>#  <files-path name="vault" path="." />\n</paths>#' "$paths_xml"
fi
# **Send to watch app** (2026-11-20): a workout file handed to the watch's app from the app's cache.
# Tauri's template already lists the cache; a template that does not gets it here. Idempotent.
if [ -f "$paths_xml" ] && ! grep -q 'cache-path' "$paths_xml"; then
    sed -i 's#</paths>#  <cache-path name="cache" path="." />\n</paths>#' "$paths_xml"
fi

echo "android-inject-service: foreground service and update download injected (AgentService + MainActivity + manifest)."
