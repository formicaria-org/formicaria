package dev.formicaria.notes

import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Base64
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.webkit.CookieManager
import android.webkit.SslErrorHandler
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebStorage
import android.webkit.WebViewClient
import android.net.http.SslError
import androidx.webkit.ScriptHandler
import androidx.media3.common.Player
import android.widget.FrameLayout
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.OnBackPressedCallback
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.webkit.WebSettingsCompat
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import java.io.File
import org.json.JSONArray
import org.json.JSONObject
import org.json.JSONTokener

class MainActivity : TauriActivity() {
  private var wv: WebView? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    watchServiceWorkers()
    // Extend into the cutout on both short edges, so the inset the window reports is the real
    // one rather than a letterboxed zero. Without this the page can be told there is nothing
    // there while the camera is sitting on top of it.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
      window.attributes.layoutInDisplayCutoutMode =
        WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
    }
    recordInstaller()
    super.onCreate(savedInstanceState)
    // Keep the app + its network alive for the study assistant's download and inference.
    val svc = Intent(this, AgentService::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) startForegroundService(svc) else startService(svc)
    takeShared(intent)
  }

  /// **Which app installed formicaria** (the installer of record), written to `cacheDir/installer`
  /// at every start and before `super.onCreate`, so the Rust side has it when it starts. An app store
  /// keeps formicaria up to date itself, so the phone's own GitHub check starts off for a store
  /// install (`decisions.md` 2026-10-30, *the Android update check follows who installed the app*);
  /// `update.rs::installer_file` reads the same literal path. Empty when Android names nobody.
  /// Failure is silent and harmless: no file means today's default, the check on.
  private fun recordInstaller() {
    try {
      val who = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        packageManager.getInstallSourceInfo(packageName).installingPackageName
      } else {
        @Suppress("DEPRECATION")
        val name = packageManager.getInstallerPackageName(packageName)
        name
      }
      File(cacheDir, "installer").writeText(who ?: "")
    } catch (e: Exception) {
    }
  }

  // `singleTask`: a share while the app is open arrives here rather than as a second app.
  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    takeShared(intent)
  }

  /// **What another app shared to formicaria** (`decisions.md#ui`, 2026-10-20): Android's Share
  /// button under a screenshot, or in the gallery. The page cannot read a `content://` address,
  /// and the app that shared it may take the permission back once it is no longer in front, so
  /// the bytes are copied at once into the app's own cache and held here until the page asks
  /// (`__fmShare`). At a cold start the page does not exist yet, which is why it asks rather than
  /// being told; `fm-share` only says there is more to ask for.
  private class Shared(val file: File, val name: String, val mime: String)
  private val shared = mutableListOf<Shared>()
  /// What could not be taken, and why: `too large` or `could not be read`.
  private val refused = mutableListOf<Pair<String, String>>()

  /// **Text brought in from another app** (`decisions.md` 2026-11-16, *the phone's Bring in*): what
  /// Share hands over as text (a page's link, a passage), and what a person selected in any app and
  /// chose *formicaria* for in the selection menu (`PROCESS_TEXT`). Held for the page like files.
  private class SharedText(val text: String, val subject: String)
  private val texts = mutableListOf<SharedText>()

  private fun takeShared(intent: Intent?) {
    val said: CharSequence? = when (intent?.action) {
      Intent.ACTION_PROCESS_TEXT -> intent.getCharSequenceExtra(Intent.EXTRA_PROCESS_TEXT)
      Intent.ACTION_SEND ->
        if (intent.type?.startsWith("text/") == true && streamOf(intent) == null) intent.getCharSequenceExtra(Intent.EXTRA_TEXT)
        else null
      else -> null
    }
    if (said != null) {
      setIntent(Intent(this, MainActivity::class.java))
      val subject = intent?.getStringExtra(Intent.EXTRA_SUBJECT) ?: ""
      val text = said.toString().take(MAX_TEXT)
      if (text.isNotBlank()) synchronized(shared) { texts.add(SharedText(text, subject)) }
      wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-share'))", null)
      return
    }
    val uris: List<Uri> = when (intent?.action) {
      Intent.ACTION_SEND -> listOfNotNull(streamOf(intent))
      Intent.ACTION_SEND_MULTIPLE -> streamsOf(intent)
      else -> return
    }
    // Taken once: a rotation recreates the activity with the intent it was started with.
    setIntent(Intent(this, MainActivity::class.java))
    if (uris.isEmpty()) return
    Thread {
      val dir = File(cacheDir, "shared").apply { mkdirs() }
      for (uri in uris) {
        val (name, size) = described(uri)
        try {
          // Read through the page as text, so a bound is kept: a phone video would not fit.
          if (size > MAX_SHARE) { synchronized(shared) { refused.add(Pair(name, "too large")) }; continue }
          // **A watch's activity file** (`decisions.md` 2026-11-20, *The phone takes watch files*):
          // a picture, or a `.fit` by name — anything else is refused before it is copied.
          val type = contentResolver.getType(uri) ?: ""
          val fit = name.lowercase().endsWith(".fit")
          if (!type.startsWith("image/") && !fit) {
            synchronized(shared) { refused.add(Pair(name, NOT_TAKEN)) }
            continue
          }
          val out = File(dir, "${System.nanoTime()}-${name.replace(Regex("[^A-Za-z0-9._-]"), "_")}")
          contentResolver.openInputStream(uri)?.use { input -> out.outputStream().use { input.copyTo(it) } }
          if (!out.isFile || out.length() > MAX_SHARE) {
            val why = if (out.isFile) "too large" else "could not be read"
            out.delete()
            synchronized(shared) { refused.add(Pair(name, why)) }
            continue
          }
          // …and by its signature: `.FIT` at bytes 8–11 of the header, as the reader checks it.
          if (fit && !type.startsWith("image/") && !fitSigned(out)) {
            out.delete()
            synchronized(shared) { refused.add(Pair(name, NOT_TAKEN)) }
            continue
          }
          val mime = if (fit && !type.startsWith("image/")) FIT_MIME else type.ifEmpty { "application/octet-stream" }
          synchronized(shared) { shared.add(Shared(out, name, mime)) }
        } catch (e: Exception) {
          // Most often the app that shared it gave no permission to read it.
          synchronized(shared) { refused.add(Pair(name, "could not be read")) }
        }
      }
      runOnUiThread { wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-share'))", null) }
    }.start()
  }

  /// The FIT header's signature, `.FIT`, at bytes 8–11.
  private fun fitSigned(f: File): Boolean = try {
    val head = ByteArray(12)
    f.inputStream().use { it.read(head) } == 12 && String(head, 8, 4, Charsets.US_ASCII) == ".FIT"
  } catch (e: Exception) { false }

  // ── A watch's files: one folder the person grants, and workouts out ─────────────────────────────
  //
  // `decisions.md` 2026-11-20, *The phone takes watch files: the share menu, and one folder the
  // person grants*. The folder is the one the watch's app (typically Gadgetbridge) exports activity
  // files into. It is granted once through Android's folder picker, **read only**; the grant is kept
  // per device (here, never in the vault). On opening and on coming back to the app it is listed
  // **off the main thread**; a `.fit` not taken before (by document, size and time) is copied into
  // the app's cache — unless its size or time changed between two looks a few seconds apart, when it
  // is still being written and waits for the next time. The page asks for what was copied
  // (`__fmWatch.pending`), imports it, and says which it has (`done`), as with a share.

  private class WatchFile(val key: String, val name: String, val file: File)
  private val watchFiles = mutableListOf<WatchFile>()
  @Volatile private var watchLooking = false
  @Volatile private var watchReadable = true
  @Volatile private var watchWaiting = 0
  @Volatile private var watchTooLarge = 0
  private fun watchPrefs() = getSharedPreferences("watch", MODE_PRIVATE)

  private val pickWatchFolder = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.OpenDocumentTree()) { uri ->
    if (uri != null) {
      try {
        contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        val old = watchPrefs().getString("tree", null)
        if (old != null && old != uri.toString()) releaseWatchFolder(Uri.parse(old))
        watchPrefs().edit().putString("tree", uri.toString()).putString("label", folderLabel(uri)).apply()
        lookWatchFolder()
      } catch (e: Exception) {
        watchReadable = false
      }
    }
    tellWatch()
  }

  private fun releaseWatchFolder(uri: Uri) {
    try { contentResolver.releasePersistableUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION) } catch (e: Exception) { }
  }

  private fun folderLabel(tree: Uri): String = try {
    val doc = android.provider.DocumentsContract.buildDocumentUriUsingTree(tree, android.provider.DocumentsContract.getTreeDocumentId(tree))
    contentResolver.query(doc, arrayOf(android.provider.DocumentsContract.Document.COLUMN_DISPLAY_NAME), null, null, null)
      ?.use { c -> if (c.moveToFirst()) c.getString(0) else null } ?: "the folder"
  } catch (e: Exception) { "the folder" }

  private class Listed(val key: String, val id: String, val name: String, val size: Long)

  /// The folder's `.fit` files, one level of subfolders deep; `null` when it cannot be read.
  private fun listWatchFolder(tree: Uri): List<Listed>? = try {
    val out = mutableListOf<Listed>()
    fun walk(parent: String, depth: Int) {
      val children = android.provider.DocumentsContract.buildChildDocumentsUriUsingTree(tree, parent)
      val cols = arrayOf(
        android.provider.DocumentsContract.Document.COLUMN_DOCUMENT_ID,
        android.provider.DocumentsContract.Document.COLUMN_DISPLAY_NAME,
        android.provider.DocumentsContract.Document.COLUMN_SIZE,
        android.provider.DocumentsContract.Document.COLUMN_LAST_MODIFIED,
        android.provider.DocumentsContract.Document.COLUMN_MIME_TYPE,
      )
      val dirs = mutableListOf<String>()
      contentResolver.query(children, cols, null, null, null)?.use { c ->
        while (c.moveToNext() && out.size < MAX_WATCH_LISTED) {
          val id = c.getString(0) ?: continue
          val name = c.getString(1) ?: ""
          val mime = c.getString(4) ?: ""
          if (mime == android.provider.DocumentsContract.Document.MIME_TYPE_DIR) { dirs.add(id); continue }
          if (!name.lowercase().endsWith(".fit")) continue
          val size = if (c.isNull(2)) -1L else c.getLong(2)
          val time = if (c.isNull(3)) -1L else c.getLong(3)
          out.add(Listed("$id|$size|$time", id, name, size))
        }
      } ?: throw IllegalStateException("no answer")
      if (depth < 1) for (d in dirs) walk(d, depth + 1)
    }
    walk(android.provider.DocumentsContract.getTreeDocumentId(tree), 0)
    out
  } catch (e: Exception) { null }

  /// List the granted folder and copy what is new into the cache — **off the main thread**.
  private fun lookWatchFolder() {
    val tree = watchPrefs().getString("tree", null) ?: return
    // Asked while a look is under way (a timed look after *Sync now* meeting a return to the app):
    // one more look after it, so a file written meanwhile is not left until the next opening.
    if (watchLooking) { watchLookAgain = true; return }
    watchLooking = true
    watchLookAgain = false
    Thread {
      try {
        val uri = Uri.parse(tree)
        val granted = contentResolver.persistedUriPermissions.any { it.uri == uri && it.isReadPermission }
        val first = if (granted) listWatchFolder(uri) else null
        watchReadable = first != null
        if (first == null) return@Thread
        val seen = watchPrefs().getStringSet("seen", emptySet())!!
        val held = synchronized(watchFiles) { watchFiles.map { it.key }.toSet() }
        val fresh = first.filter { it.key !in seen && it.key !in held }
        if (fresh.isEmpty()) { watchWaiting = 0; return@Thread }
        // A second look a few seconds later: a file still being written has another size or time.
        Thread.sleep(WATCH_SETTLE_MS)
        val again = (listWatchFolder(uri) ?: emptyList()).map { it.key }.toSet()
        val settled = fresh.filter { it.key in again }
        watchWaiting = fresh.size - settled.size
        val dir = File(cacheDir, "watch").apply { mkdirs() }
        var large = 0
        for (f in settled) {
          if (f.size > MAX_SHARE) { large++; continue }
          val out = File(dir, "${System.nanoTime()}.fit")
          try {
            val doc = android.provider.DocumentsContract.buildDocumentUriUsingTree(uri, f.id)
            contentResolver.openInputStream(doc)?.use { input -> out.outputStream().use { input.copyTo(it) } }
            if (out.isFile && out.length() <= MAX_SHARE) {
              synchronized(watchFiles) { watchFiles.add(WatchFile(f.key, f.name, out)) }
            } else { out.delete(); large++ }
          } catch (e: Exception) { out.delete() }
        }
        watchTooLarge = large
      } finally {
        watchLooking = false
        tellWatch()
        if (watchLookAgain) runOnUiThread { lookWatchFolder() }
      }
    }.start()
  }

  // ── The watch's app: Gadgetbridge, set up and asked from here ───────────────────────────────────
  //
  // `decisions.md` 2026-11-20 (night), *formicaria sets up Gadgetbridge and asks it to sync*. fm
  // still never talks to the watch: Gadgetbridge does, and writes the files fm reads. What fm adds
  // is knowing whether Gadgetbridge is installed (the manifest's `<queries>` names its packages, and
  // only those), opening it or its F-Droid page, and one message of Gadgetbridge's own Intent API,
  // which the person switches on in Gadgetbridge: **"Allow activity sync trigger"** (fm's *Sync
  // now* sends `ACTIVITY_SYNC` to Gadgetbridge's package). Gadgetbridge can also announce the end
  // of a sync, but hearing it takes a receiver, and the APK carries none (Play Protect, 2026-10-31,
  // `ci/checks.sh`): the folder is looked at a few times after *Sync now* instead, and as before on
  // every opening and return to the app.
  @Volatile private var watchLookAgain = false
  @Volatile private var watchAppAsked = 0L

  /// The installed Gadgetbridge's package (release first, then the nightly builds), or `null`.
  @Suppress("DEPRECATION")
  private fun watchApp(): String? = GB_PACKAGES.firstOrNull {
    try { packageManager.getPackageInfo(it, 0); true } catch (e: Exception) { false }
  }

  private fun tellWatch() {
    runOnUiThread { wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-watch'))", null) }
  }

  /// The workout file the page asked to save, held while Android's *Save as* is open.
  private var workoutToSave: ByteArray? = null
  private val saveWorkout = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.CreateDocument("application/octet-stream")) { uri ->
    val bytes = workoutToSave
    workoutToSave = null
    var ok = false
    if (uri != null && bytes != null) {
      try { contentResolver.openOutputStream(uri)?.use { it.write(bytes) }; ok = true } catch (e: Exception) { }
    }
    val d = JSONObject().put("saved", ok).put("chosen", uri != null)
    wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-workout-saved',{detail:$d}))", null)
  }

  /// The page's side of the watch's files.
  inner class WatchShell {
    /// `{folder, readable, looking, held, waiting, tooLarge}`; `folder` is empty with no grant.
    @JavascriptInterface
    fun state(): String {
      val p = watchPrefs()
      return JSONObject()
        .put("folder", if (p.getString("tree", null) != null) p.getString("label", "the folder") else "")
        .put("readable", watchReadable)
        .put("looking", watchLooking)
        .put("held", synchronized(watchFiles) { watchFiles.size })
        .put("waiting", watchWaiting)
        .put("tooLarge", watchTooLarge)
        .put("app", watchApp() ?: "")
        .put("asked", watchAppAsked)
        .toString()
    }

    /// Open Gadgetbridge (its own screen, to pair the watch or change its settings).
    @JavascriptInterface
    fun openApp() {
      runOnUiThread {
        try {
          val pkg = watchApp() ?: return@runOnUiThread
          packageManager.getLaunchIntentForPackage(pkg)?.let { startActivity(it) }
        } catch (e: Exception) { }
      }
    }

    /// Gadgetbridge's page on F-Droid, where it is published (the F-Droid app takes the link when
    /// it is installed, the browser otherwise).
    @JavascriptInterface
    fun getApp() {
      runOnUiThread {
        try { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(GB_STORE))) } catch (e: Exception) { }
      }
    }

    /// **Sync now**: ask Gadgetbridge to fetch the watch's new activities. Gadgetbridge ignores it
    /// unless its *Allow activity sync trigger* is on; nothing comes back to say so. The folder is
    /// looked at a few times over the next minutes, while Gadgetbridge syncs and exports.
    @JavascriptInterface
    fun sync() {
      val pkg = watchApp() ?: return
      try {
        sendBroadcast(Intent(GB_SYNC).setPackage(pkg))
        watchAppAsked = System.currentTimeMillis()
      } catch (e: Exception) { return }
      val main = android.os.Handler(android.os.Looper.getMainLooper())
      for (ms in WATCH_SYNC_LOOKS) main.postDelayed({ lookWatchFolder() }, ms)
      tellWatch()
    }

    @JavascriptInterface
    fun choose() { runOnUiThread { try { pickWatchFolder.launch(null) } catch (e: Exception) { } } }

    /// Stop reading the folder: the grant is given back and nothing more is taken from it.
    @JavascriptInterface
    fun forget() {
      val tree = watchPrefs().getString("tree", null)
      if (tree != null) releaseWatchFolder(Uri.parse(tree))
      watchPrefs().edit().remove("tree").remove("label").remove("seen").apply()
      synchronized(watchFiles) { watchFiles.forEach { it.file.delete() }; watchFiles.clear() }
      watchReadable = true; watchWaiting = 0; watchTooLarge = 0
      tellWatch()
    }

    @JavascriptInterface
    fun look() = lookWatchFolder()

    @JavascriptInterface
    fun pending(): String = synchronized(watchFiles) {
      val a = JSONArray()
      for (f in watchFiles) a.put(JSONObject().put("key", f.key).put("name", f.name))
      a.toString()
    }

    @JavascriptInterface
    fun read(key: String): String {
      val f = synchronized(watchFiles) { watchFiles.firstOrNull { it.key == key } } ?: return ""
      return Base64.encodeToString(f.file.readBytes(), Base64.NO_WRAP)
    }

    /// The page imported these (or the import refused them): never taken again.
    @JavascriptInterface
    fun done(keys: String) {
      val list = try { JSONArray(keys) } catch (e: Exception) { return }
      val taken = (0 until list.length()).map { list.optString(it) }.toSet()
      synchronized(watchFiles) {
        watchFiles.filter { it.key in taken }.forEach { it.file.delete() }
        watchFiles.removeAll { it.key in taken }
      }
      val seen = watchPrefs().getStringSet("seen", emptySet())!!.toMutableSet()
      seen.addAll(taken)
      watchPrefs().edit().putStringSet("seen", seen.toList().takeLast(MAX_WATCH_SEEN).toSet()).apply()
    }

    /// **Send a workout to the watch's app** (S.7): Android's share sheet with the file, for the
    /// person to pick their watch's app (Gadgetbridge installs a workout file it is given).
    @JavascriptInterface
    fun send(name: String, data: String) {
      runOnUiThread {
        try {
          val dir = File(cacheDir, "workouts").apply { mkdirs() }
          dir.listFiles()?.forEach { it.delete() }
          val f = File(dir, name.replace(Regex("[^A-Za-z0-9._-]"), "_").ifEmpty { "workout.fit" })
          f.writeBytes(Base64.decode(data, Base64.DEFAULT))
          val uri = androidx.core.content.FileProvider.getUriForFile(this@MainActivity, "$packageName.fileprovider", f)
          val send = Intent(Intent.ACTION_SEND).setType("application/octet-stream")
            .putExtra(Intent.EXTRA_STREAM, uri)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
          send.clipData = android.content.ClipData.newRawUri(f.name, uri)
          startActivity(Intent.createChooser(send, "Send to your watch app"))
        } catch (e: Exception) { }
      }
    }

    /// **Save a workout file** where the person chooses (Android's *Save as*).
    @JavascriptInterface
    fun save(name: String, data: String) {
      runOnUiThread {
        try {
          workoutToSave = Base64.decode(data, Base64.DEFAULT)
          saveWorkout.launch(name)
        } catch (e: Exception) { workoutToSave = null }
      }
    }
  }

  @Suppress("DEPRECATION")
  private fun streamOf(intent: Intent): Uri? =
    if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra(Intent.EXTRA_STREAM, Uri::class.java)
    else intent.getParcelableExtra(Intent.EXTRA_STREAM)

  @Suppress("DEPRECATION")
  private fun streamsOf(intent: Intent): List<Uri> =
    (if (Build.VERSION.SDK_INT >= 33) intent.getParcelableArrayListExtra(Intent.EXTRA_STREAM, Uri::class.java)
    else intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM)) ?: emptyList()

  /// The file's own name and size, as the app that shared it gives them; -1 when it does not say.
  private fun described(uri: Uri): Pair<String, Long> {
    var name = uri.lastPathSegment ?: "shared"
    var size = -1L
    try {
      contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE), null, null, null)
        ?.use { c ->
          if (c.moveToFirst()) {
            val n = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            val s = c.getColumnIndex(OpenableColumns.SIZE)
            if (n >= 0 && !c.isNull(n)) name = c.getString(n)
            if (s >= 0 && !c.isNull(s)) size = c.getLong(s)
          }
        }
    } catch (e: Exception) {
      // Some apps share without answering the query; the copy still goes ahead.
    }
    return Pair(name, size)
  }

  /// The page's side of a share: what is held, each file's bytes, and that it has them. `done`
  /// takes counts, not "all": a second share still being copied while the page reads the first
  /// must not be thrown away with it.
  inner class Share {
    /// **Open with…** (2026-11-17): a song's or a video's file handed to another app to play — the
    /// vault's own file, read through a content link, never copied. Only files inside the app's
    /// storage are offered (`file_paths.xml`'s `files-path`).
    @JavascriptInterface
    fun openWith(path: String, mime: String) {
      runOnUiThread {
        try {
          val f = File(path)
          if (!f.canonicalPath.startsWith(filesDir.canonicalPath)) return@runOnUiThread
          val uri = androidx.core.content.FileProvider.getUriForFile(this@MainActivity, "$packageName.fileprovider", f)
          val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, mime.ifEmpty { "*/*" })
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
          startActivity(Intent.createChooser(view, "Open with"))
        } catch (e: Exception) { }
      }
    }

    @JavascriptInterface
    fun pending(): String = synchronized(shared) {
      val files = JSONArray()
      for (f in shared) files.put(JSONObject().put("name", f.name).put("mime", f.mime))
      val no = JSONArray()
      for ((name, why) in refused) no.put(JSONObject().put("name", name).put("why", why))
      val said = JSONArray()
      for (t in texts) said.put(JSONObject().put("text", t.text).put("subject", t.subject))
      val got = JSONArray()
      synchronized(brought) { for (b in brought) got.put(b) }
      JSONObject().put("files", files).put("refused", no).put("texts", said).put("brought", got).toString()
    }

    /// **A note handed to another app** (`workspace-design` §13.6): Android's share sheet with the note's
    /// text; the person picks where it goes.
    @JavascriptInterface
    fun out(text: String, subject: String) {
      val send = Intent(Intent.ACTION_SEND).setType("text/plain")
        .putExtra(Intent.EXTRA_TEXT, text.take(MAX_TEXT)).putExtra(Intent.EXTRA_SUBJECT, subject)
      runOnUiThread { try { startActivity(Intent.createChooser(send, null)) } catch (e: Exception) { } }
    }

    /// The page has the pieces brought from a page screen (by count).
    @JavascriptInterface
    fun doneBrought(n: Int) {
      synchronized(brought) { repeat(minOf(n, brought.size)) { brought.removeAt(0) } }
    }

    /// The page has the texts it was given (by count, as for files).
    @JavascriptInterface
    fun doneTexts(n: Int) {
      synchronized(shared) { repeat(minOf(n, texts.size)) { texts.removeAt(0) } }
    }

    @JavascriptInterface
    fun read(i: Int): String {
      val f = synchronized(shared) { shared.getOrNull(i) } ?: return ""
      return Base64.encodeToString(f.file.readBytes(), Base64.NO_WRAP)
    }

    @JavascriptInterface
    fun done(files: Int, refusedCount: Int) {
      synchronized(shared) {
        repeat(minOf(files, shared.size)) { shared.removeAt(0).file.delete() }
        repeat(minOf(refusedCount, refused.size)) { refused.removeAt(0) }
      }
    }
  }

  /// The insets the window last reported, in CSS pixels. Held so the page can **ask** for them.
  // `Float`, because `displayMetrics.density` is one and the division follows it.
  private var top = 0f
  private var right = 0f
  private var bottom = 0f
  private var left = 0f
  /// The on-screen keyboard **beyond the navigation bar**, in CSS pixels; 0 while it is closed.
  ///
  /// Edge-to-edge at SDK 35+ means Android no longer resizes the window for the keyboard — it is drawn
  /// over the WebView, and the page believes the whole screen is visible, so the line being typed sat
  /// under it (the owner, 2026-10-08). The page subtracts this from its height (`--app-h`). Beyond the
  /// navigation bar because the page already pays that as `--safe-bottom`.
  private var keyboard = 0f

  /// **The page asks; the shell does not tell.**
  ///
  /// `env(safe-area-inset-*)` is 0 in wry's WebView — it never forwards the window insets — so the
  /// stylesheet was guessing: 1.75rem against a camera cutout that is really 52px, and 0.5rem
  /// against a navigation bar that is really 47px.
  ///
  /// The first attempt at this pushed the values in with `evaluateJavascript` from the inset
  /// listener, and it never landed. `onWebViewCreate` runs *before* wry issues the first
  /// `loadUrl` and before `setContentView`, so the first inset dispatch — and the `post` after it
  /// — happen against `about:blank`, and the inline properties die when the real document commits.
  /// It looked intermittent because it was a race.
  ///
  /// So this copies the shape `FM_CONFIG_DIR` already uses: **put the platform fact where the
  /// consumer already looks, and let the consumer read it when it is ready.** A registered
  /// JavaScript interface is re-injected into every document Android loads, so it is present for
  /// the first page and survives every reload — and wry uses exactly this mechanism for its own
  /// IPC bridge, so it is native to this stack rather than a new idea.
  /// **The device's own status-bar height, in CSS pixels** — the fallback, and not a guess.
  ///
  /// **A zero here defeated the stylesheet's fallback rather than falling back to it**, which is
  /// the whole bug and is worth stating exactly. `top` is 0 until the first inset dispatch, and
  /// `addDocumentStartJavaScript` runs `APPLY_INSETS` at *document start* — so the page opened with
  /// `--safe-top: 0px` written as an **inline style on `:root`**, and an inline style outranks the
  /// `@media (pointer: coarse)` floor completely. The floor was therefore never in force on Android
  /// at all: not 28px, but nothing. Confirmed against the shipped layout — the top control rendered
  /// at `y = 0`, its full height inside a camera cutout that measures `DisplayCutout
  /// insets=Rect(0, 130 - 0, 0)` / density 3.25 = **40 CSS px**. Reported 2026-09-08:
  /// *"you are continuing to use the top part of the screen which is untouchable and passes through
  /// the frontal camera… we cannot use top pixels."*
  ///
  /// The general shape, worth more than the instance: **a bridge that publishes a placeholder
  /// overrides the fallback it was meant to complement.** Either report nothing until you know, or
  /// report something true — never a zero that outranks a guess.
  ///
  /// Android has always known the real number, so ask it rather than picking one: `status_bar_height`
  /// is a platform dimen resource, available immediately and correct per device. A phone whose
  /// status bar is shorter than its cutout would still be wrong, so the cutout is unioned in where
  /// the API exists — this returns the larger of the two, which is what `systemBars() or
  /// displayCutout()` will report once it does dispatch.
  private fun fallbackTop(): Float {
    val d = resources.displayMetrics.density
    val id = resources.getIdentifier("status_bar_height", "dimen", "android")
    val bar = if (id > 0) resources.getDimensionPixelSize(id) / d else 0f
    val cut =
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        (window?.decorView?.rootWindowInsets?.displayCutout?.safeInsetTop ?: 0) / d
      } else {
        0f
      }
    return maxOf(bar, cut)
  }

  inner class Insets {
    @JavascriptInterface
    fun json(): String {
      // **Never report a zero top before the first dispatch.** Zero is indistinguishable from
      // "this device has no status bar", and the page cannot tell the difference either — so the
      // one reading that must never be guessed downwards is guessed from the device itself.
      val t = if (top > 0f) top else fallbackTop()
      return "{\"top\":$t,\"right\":$right,\"bottom\":$bottom,\"left\":$left,\"keyboard\":$keyboard}"
    }
  }

  /// **Updating hands the newest APK to the browser, and Android's installer takes it from there**
  /// (`decisions.md#track-m`, 2026-10-31, *a phone update opens in the browser*).
  ///
  /// The app used to download, verify and install its own update, which needs
  /// `REQUEST_INSTALL_PACKAGES` — a permission scanners read as a red flag, and the next
  /// suspect once the restart receiver alone did not clear *"Harmful app blocked"*. Now this opens
  /// one fixed address, the newest release's APK, in the browser; the person taps **Open** when it
  /// arrives, exactly as on the first install (`docs/src/user/android.md`). Android still refuses an
  /// update signed by anyone else.
  ///
  /// **It takes no argument**, as before: nothing running in the page can name another address.
  /// Returns `ok` once the browser has been asked. A phone with no browser at all is not reported:
  /// the failure happens on the UI thread, after this has answered.
  inner class UpdateDownload {
    @JavascriptInterface
    fun download(): String {
      val view = Intent(Intent.ACTION_VIEW, Uri.parse(LATEST_APK)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      runOnUiThread { try { startActivity(view) } catch (e: Exception) { } }
      return "ok"
    }
  }

  /// **A web page, an item of the collection** (`docs/context/workspace-design-2026-11-15.md` §13.7):
  /// the app owns the collection, the switch and the layout; the shell only **draws** a page's
  /// WebView over the box the app gives (CSS pixels of the app's own view), hides it when the app
  /// says it is out of sight, and tells the app the page's title and address as they change. One
  /// WebView per item, kept while the item is open, so a page switched away from and back is where
  /// it was. A page is the web's: no bridge into it, nothing of formicaria's inside it.
  private val pages = HashMap<String, WebView>()
  /** The page item drawn last (the one in front when two are shown together). */
  private var lastShown: String? = null

  /** What the protections did on each page (the Privacy sheet), and the scripts each page got. */
  private val counts = HashMap<String, PageCounts>()
  private val scripts = HashMap<String, List<ScriptHandler>>()
  /** The media files and streams each page fetched (Save media), the last 200. */
  private val seenMedia = java.util.concurrent.ConcurrentHashMap<String, java.util.concurrent.ConcurrentLinkedDeque<String>>()
  private val MEDIA_URL = Regex("\\.(m3u8|mpd|mp4|m4v|webm|mov|mp3|m4a|aac|ogg|oga|opus|flac|wav)(\\?|$)", RegexOption.IGNORE_CASE)
  /** An `https` address formicaria tried in place of an `http` one → the `http` one. */
  private val upgradedFrom = java.util.concurrent.ConcurrentHashMap<String, String>()

  /** Each page's address, readable from the threads its requests are asked on. */
  private val pageUrls = java.util.concurrent.ConcurrentHashMap<String, String>()
  private val countsDue = HashSet<String>()
  /** The count told at most twice a second, however many requests are refused. */
  private fun tellCountsSoon(id: String) {
    if (!countsDue.add(id)) return
    wv?.postDelayed({ countsDue.remove(id); tellCounts(id) }, 500)
  }

  private fun tellCounts(id: String) {
    val d = (counts[id] ?: return).json(id)
    wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-privacy',{detail:$d}))", null)
  }

  /** The address a page should really open: tracking parameters off, made secure. Null if as is. */
  private fun rewrite(id: String, url: String): String? {
    val c = counts.getOrPut(id) { PageCounts() }
    var next = url
    PagePrivacy.clean(next)?.let { next = it; c.cleaned++ }
    PagePrivacy.upgrade(next)?.let { upgradedFrom[it] = next; next = it; c.upgraded++ }
    if (next == url) return null
    tellCounts(id)
    return next
  }

  /** **A secure address that does not answer gives way in seconds** (2026-11-22; the owner: some
   *  homepages "do not open cleanly"). A site with no https at all (a server that never answers on
   *  443) left the page blank until Android gave up on the connection, a minute or more, before the
   *  choice to open it anyway appeared. After 8 s with nothing shown, that choice is shown. A page
   *  that shows itself in time is forgotten here (`onPageCommitVisible`). */
  private fun watchUpgrade(view: WebView, https: String) {
    if (!upgradedFrom.containsKey(https)) return
    view.postDelayed({
      val http = upgradedFrom.remove(https) ?: return@postDelayed
      if (view.url != null && view.url != https && !view.url!!.startsWith("about:")) return@postDelayed
      view.stopLoading()
      view.loadDataWithBaseURL(null, PagePrivacy.insecurePage(http), "text/html", "utf-8", null)
    }, 8000)
  }

  /** **A PDF opens in the phone's PDF app** (2026-11-22; the owner: arXiv papers "do not open
   *  cleanly"). Android's page view cannot show a PDF: a link to one did nothing. It is fetched
   *  with the page's own cookies into the app's storage (the newest 20 kept) and handed to whichever
   *  app the person uses for PDFs, as Chrome on Android does. */
  private fun openPdf(url: String, ua: String?) {
    Thread {
      try {
        val dir = File(filesDir, "web-pdf").apply { mkdirs() }
        dir.listFiles()?.sortedByDescending { it.lastModified() }?.drop(20)?.forEach { it.delete() }
        var u = java.net.URL(url)
        var conn: java.net.HttpURLConnection
        var hops = 0
        while (true) {
          conn = u.openConnection() as java.net.HttpURLConnection
          conn.instanceFollowRedirects = false
          conn.connectTimeout = 15000
          conn.readTimeout = 30000
          CookieManager.getInstance().getCookie(u.toString())?.let { conn.setRequestProperty("Cookie", it) }
          if (!ua.isNullOrEmpty()) conn.setRequestProperty("User-Agent", ua)
          val code = conn.responseCode
          if (code in 300..399 && hops < 5) {
            u = java.net.URL(u, conn.getHeaderField("Location") ?: break)
            conn.disconnect(); hops++; continue
          }
          break
        }
        val base = (u.path.substringAfterLast('/').ifEmpty { "paper" }).replace(Regex("[^A-Za-z0-9._-]"), "_").take(80)
        val f = File(dir, if (base.endsWith(".pdf", true)) base else "$base.pdf")
        conn.inputStream.use { inp ->
          f.outputStream().use { out ->
            val buf = ByteArray(64 * 1024); var total = 0L
            while (true) {
              val n = inp.read(buf); if (n < 0) break
              total += n; if (total > 150L * 1024 * 1024) throw java.io.IOException("too large")
              out.write(buf, 0, n)
            }
          }
        }
        runOnUiThread {
          try {
            val uri = androidx.core.content.FileProvider.getUriForFile(this, "$packageName.fileprovider", f)
            val view = Intent(Intent.ACTION_VIEW).setDataAndType(uri, "application/pdf")
              .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            startActivity(Intent.createChooser(view, "Open the PDF with"))
          } catch (e: Exception) {
            android.widget.Toast.makeText(this, "No app on this phone opens PDFs", android.widget.Toast.LENGTH_LONG).show()
          }
        }
      } catch (e: Exception) {
        runOnUiThread {
          android.widget.Toast.makeText(this, "The PDF could not be fetched", android.widget.Toast.LENGTH_LONG).show()
        }
      }
    }.start()
  }

  private fun open(id: String, page: WebView, url: String) {
    val next = rewrite(id, url) ?: url
    scriptletsFor(id, page, next)
    page.loadUrl(next, PagePrivacy.headers(next))
    watchUpgrade(page, next)
  }

  /** **A site's scriptlets** (`privacy-plan-2026-11-19.md` 3.3), registered for its origin before
   *  its page opens: the page view runs them at the document's start, so the site's own rules
   *  (its CSP) cannot refuse them. Once per origin and page; again when the switches change. */
  private val scriptlets = HashMap<String, HashMap<String, ScriptHandler>>()
  private fun scriptletsFor(id: String, page: WebView, url: String) {
    if (!(PagePrivacy.trackers || PagePrivacy.ads) || PagePrivacy.off(url)) return
    if (!WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) return
    val u = try { Uri.parse(url) } catch (e: Exception) { return }
    if (u.scheme != "https" && u.scheme != "http") return
    val origin = "${u.scheme}://${u.authority}"
    val mine = scriptlets.getOrPut(id) { HashMap() }
    if (origin in mine) return
    val code = try { Blocker.scriptlets(url) } catch (e: Throwable) { "" }
    if (code.isEmpty()) return
    try { mine[origin] = WebViewCompat.addDocumentStartJavaScript(page, code, setOf(origin)) } catch (e: Exception) { }
  }

  /** **A saved page** (a note's copy of one): read from the vault's own file, shown as it was saved,
   *  at its own address, so its links lead where they led. Only files in the app's storage. */
  private fun openSaved(page: WebView, path: String) {
    try {
      val f = File(path)
      if (!f.canonicalPath.startsWith(filesDir.canonicalPath) || f.length() > 60L * 1024 * 1024) return
      val html = f.readText()
      val base = Regex("<base[^>]+href=[\"']([^\"']+)", RegexOption.IGNORE_CASE).find(html)?.groupValues?.get(1)
      page.loadDataWithBaseURL(base, html, "text/html", "utf-8", null)
    } catch (e: Exception) { }
  }

  /** **A page's service workers ask too** (`privacy-plan-2026-11-19.md` 2.1): their requests reach
   *  no page view's client, so they are checked here, against the page that made them (`Referer`,
   *  else the page last opened). Set before any page view exists, as Android requires. */
  private fun watchServiceWorkers() {
    if (!WebViewFeature.isFeatureSupported(WebViewFeature.SERVICE_WORKER_BASIC_USAGE) ||
      !WebViewFeature.isFeatureSupported(WebViewFeature.SERVICE_WORKER_SHOULD_INTERCEPT_REQUEST)) return
    try {
      androidx.webkit.ServiceWorkerControllerCompat.getInstance().setServiceWorkerClient(
        object : androidx.webkit.ServiceWorkerClientCompat() {
          override fun shouldInterceptRequest(req: WebResourceRequest): android.webkit.WebResourceResponse? {
            if (!(PagePrivacy.trackers || PagePrivacy.ads)) return null
            val h = req.requestHeaders
            val page = h["Referer"] ?: h["referer"] ?: pageUrls.values.lastOrNull() ?: return null
            if (PagePrivacy.off(page)) return null
            val u = req.url.toString()
            val kind = Blocker.kind(u, h["Accept"] ?: h["accept"], false, h["Sec-Fetch-Dest"] ?: h["sec-fetch-dest"])
            return Blocker.response(try { Blocker.verdict(u, page, kind) } catch (e: Throwable) { "" })
          }
        })
    } catch (e: Exception) { }
  }

  /** The scripts a page gets at its start, as the switches stand now (re-given when they change). */
  private fun giveScripts(id: String, page: WebView) {
    scripts.remove(id)?.forEach { it.remove() }
    scriptlets.remove(id)?.values?.forEach { it.remove() }
    page.url?.let { scriptletsFor(id, page, it) }
    if (!WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) return
    scripts[id] = PagePrivacy.startScripts().map { WebViewCompat.addDocumentStartJavaScript(page, it, setOf("*")) }
  }

  /** The fire button: every page's cookies, stored data, cache and history, not formicaria's own. */
  private fun forget() {
    CookieManager.getInstance().removeAllCookies(null)
    CookieManager.getInstance().flush()
    val mine = wv?.url?.let { Uri.parse(it) }?.let { "${it.scheme}://${it.authority}" }
    WebStorage.getInstance().getOrigins { m ->
      for (o in (m as? Map<*, *>)?.keys ?: emptySet<Any>()) {
        val s = o.toString()
        if (mine == null || !s.startsWith(mine)) WebStorage.getInstance().deleteOrigin(s)
      }
    }
    pages.values.forEach { it.clearCache(true); it.clearHistory(); it.clearFormData() }
    // A new session for the fingerprinting defence: sites' readings no longer match the old ones.
    PagePrivacy.reseed()
    android.webkit.WebViewDatabase.getInstance(this).clearHttpAuthUsernamePassword()
    counts.clear()
  }

  /** A page's own counter: the cookie pop-up script tells it each banner refused. Nothing else. */
  inner class PageCounter(private val id: String) {
    @JavascriptInterface
    fun banner() {
      runOnUiThread { counts.getOrPut(id) { PageCounts() }.banners++; tellCounts(id) }
    }

    /// The lists' hide rules for the page (its own) and for the names it uses (the generic ones).
    @JavascriptInterface
    fun hide(url: String): String = if (PagePrivacy.off(url)) "" else try { Blocker.hide(url) } catch (e: Throwable) { "" }

    @JavascriptInterface
    fun hideNames(url: String, classes: String, ids: String): String =
      if (PagePrivacy.off(url)) "" else try { Blocker.hideNames(url, classes, ids) } catch (e: Throwable) { "" }

    /// The site's procedural hide rules (`ui/page/procedural.ts` applies them), a JSON array.
    @JavascriptInterface
    fun procedural(url: String): String = if (PagePrivacy.off(url)) "[]" else try { Blocker.procedural(url) } catch (e: Throwable) { "[]" }
  }

  private fun tellApp(id: String, page: WebView) {
    val d = JSONObject().put("id", id).put("title", page.title ?: "").put("url", page.url ?: "")
    wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page',{detail:$d}))", null)
  }

  private fun pageFor(id: String, url: String): WebView {
    pages[id]?.let { return it }
    val page = WebView(this)
    page.settings.javaScriptEnabled = true
    page.settings.domStorageEnabled = true
    page.settings.builtInZoomControls = true
    page.settings.displayZoomControls = false
    // **What the page view tells sites** (`privacy-plan-2026-11-19.md` 2.3): not this app's name
    // (Android's `X-Requested-With`), and no cookies for other sites inside a page.
    if (WebViewFeature.isFeatureSupported(WebViewFeature.REQUESTED_WITH_HEADER_ALLOW_LIST))
      try { WebSettingsCompat.setRequestedWithHeaderOriginAllowList(page.settings, emptySet()) } catch (e: Exception) { }
    CookieManager.getInstance().setAcceptThirdPartyCookies(page, false)
    page.setDownloadListener { url, ua, disposition, mime, _ ->
      val pdf = mime?.contains("pdf", true) == true || url.substringBefore('?').endsWith(".pdf", true) ||
        disposition?.contains(".pdf", true) == true
      if (pdf) openPdf(url, ua)
    }
    page.webViewClient = object : WebViewClient() {
      override fun doUpdateVisitedHistory(view: WebView, u: String?, reload: Boolean) {
        pageUrls[id] = view.url ?: ""
        tellApp(id, view)
      }

      override fun onPageStarted(view: WebView, u: String?, icon: android.graphics.Bitmap?) {
        pageUrls[id] = u ?: ""
        counts[id]?.let { it.blocked = 0; it.banners = 0 }
      }

      /// A link followed: tracking parameters off and made secure (the Privacy switches), and the
      /// one way back to a site without a secure version, chosen on its page.
      override fun shouldOverrideUrlLoading(view: WebView, req: WebResourceRequest): Boolean {
        val u = req.url.toString()
        if (u.startsWith("fm-insecure:")) {
          val http = u.removePrefix("fm-insecure:")
          PagePrivacy.insecureOk += PagePrivacy.host(http)
          view.loadUrl(http, PagePrivacy.headers(http))
          return true
        }
        if (!req.isForMainFrame || req.method != "GET") return false
        val next = rewrite(id, u)
        scriptletsFor(id, view, next ?: u)
        if (next == null) return false
        view.loadUrl(next, PagePrivacy.headers(next))
        watchUpgrade(view, next)
        return true
      }

      override fun onPageCommitVisible(view: WebView, u: String?) {
        // The secure page showed itself: it needs no way back.
        if (u != null) upgradedFrom.remove(u)
      }

      /// Every request the page makes: media ones are remembered for Save media.
      override fun shouldInterceptRequest(view: WebView, req: WebResourceRequest): android.webkit.WebResourceResponse? {
        val u = req.url.toString()
        val pageUrl = pageUrls[id] ?: ""
        if ((PagePrivacy.trackers || PagePrivacy.ads) && !PagePrivacy.off(pageUrl) && !req.isForMainFrame) {
          val h = req.requestHeaders
          val kind = Blocker.kind(u, h["Accept"] ?: h["accept"], false, h["Sec-Fetch-Dest"] ?: h["sec-fetch-dest"])
          val answer = Blocker.response(try { Blocker.verdict(u, pageUrl, kind) } catch (e: Throwable) { "" })
          if (answer != null) {
            runOnUiThread { counts.getOrPut(id) { PageCounts() }.blocked++; tellCountsSoon(id) }
            return answer
          }
        }
        if (MEDIA_URL.containsMatchIn(u)) {
          val q = seenMedia.getOrPut(id) { java.util.concurrent.ConcurrentLinkedDeque() }
          q.add(u)
          while (q.size > 200) q.pollFirst()
        }
        return null
      }

      override fun onReceivedError(view: WebView, req: WebResourceRequest, err: WebResourceError) {
        if (!req.isForMainFrame) return
        val http = upgradedFrom.remove(req.url.toString()) ?: return
        view.loadDataWithBaseURL(null, PagePrivacy.insecurePage(http), "text/html", "utf-8", null)
      }

      override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: SslError) {
        handler.cancel()
        val http = upgradedFrom.remove(error.url ?: "") ?: return
        view.loadDataWithBaseURL(null, PagePrivacy.insecurePage(http), "text/html", "utf-8", null)
      }
    }
    page.addJavascriptInterface(PageCounter(id), "__fmPagePrivacy")
    giveScripts(id, page)
    page.webChromeClient = object : WebChromeClient() {
      override fun onReceivedTitle(view: WebView, title: String?) = tellApp(id, view)
    }
    // **A long press on an image or a link offers it** (text keeps Android's own selection): bring it
    // into the note, or open the link as a page item of its own.
    page.setOnLongClickListener {
      val hit = page.hitTestResult
      val src = hit.extra
      if (src == null || !(src.startsWith("https://") || src.startsWith("http://"))) return@setOnLongClickListener false
      when (hit.type) {
        WebView.HitTestResult.IMAGE_TYPE, WebView.HitTestResult.SRC_IMAGE_ANCHOR_TYPE -> { offer(page, "image", src); true }
        WebView.HitTestResult.SRC_ANCHOR_TYPE -> { offer(page, "link", src); true }
        else -> false
      }
    }
    page.visibility = View.GONE
    findViewById<ViewGroup>(android.R.id.content)
      .addView(page, FrameLayout.LayoutParams(0, 0))
    pages[id] = page
    if (url.startsWith("https://") || url.startsWith("http://")) open(id, page, url)
    else if (url.startsWith("fmsaved:")) openSaved(page, url.removePrefix("fmsaved:"))
    return page
  }

  private fun offer(page: WebView, kind: String, src: String) {
    val items = if (kind == "link") arrayOf("Bring the link into your note", "Open it as a page")
      else arrayOf("Bring the image into your note")
    android.app.AlertDialog.Builder(this).setItems(items) { _, which ->
      if (which == 0) {
        bring(JSONObject().put("kind", kind).put("src", src).put("pageUrl", page.url ?: "")
          .put("pageTitle", page.title ?: "").put("note", ""))
      } else {
        val d = JSONObject().put("url", src)
        wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-open',{detail:$d}))", null)
      }
    }.show()
  }

  /// **Bring into note, first in Android's selection bar** (`decisions.md` 2026-11-17, *one frame*;
  /// the owner: *"on the phone I have it not as the main thing, only after scrolling the options"*).
  /// For a selection in formicaria's own screens or in a page it draws, the item goes in ahead of the
  /// system's; from other apps Android orders outside items late, which formicaria cannot change.
  override fun onActionModeStarted(mode: android.view.ActionMode) {
    super.onActionModeStarted(mode)
    val menu = mode.menu
    if (menu.findItem(BRING_ITEM) != null) return
    val page = pages.entries.firstOrNull { it.value.visibility == View.VISIBLE && it.value.hasFocus() }?.key
    menu.add(Menu.NONE, BRING_ITEM, 0, "Bring into note")
      .setShowAsActionFlags(MenuItem.SHOW_AS_ACTION_ALWAYS)
      .setOnMenuItemClickListener {
        if (page != null) Pages().bring(page, "")
        else wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-bring-selection'))", null)
        mode.finish()
        true
      }
  }

  /// **Trackers in other apps** (`TrackerVpn`): Android asks the person once; then the switch starts
  /// and stops it. The apps let through are kept here, and the block restarts without them.
  private val vpnAsk = registerForActivityResult(androidx.activity.result.contract.ActivityResultContracts.StartActivityForResult()) { r ->
    if (r.resultCode == RESULT_OK) startService(Intent(this, TrackerVpn::class.java))
    else wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-vpn',{detail:{refused:true}}))", null)
  }
  private fun vpnWanted(on: Boolean) {
    TrackerVpn.allowed = getSharedPreferences("vpn", MODE_PRIVATE).getStringSet("allowed", emptySet())!!.toSet()
    if (on == TrackerVpn.on) return
    if (!on) { startService(Intent(this, TrackerVpn::class.java).setAction(TrackerVpn.STOP)); return }
    val ask = android.net.VpnService.prepare(this)
    if (ask != null) vpnAsk.launch(ask) else startService(Intent(this, TrackerVpn::class.java))
  }
  inner class VpnShell {
    @JavascriptInterface
    fun state(): String = TrackerVpn.state(this@MainActivity) { pkg ->
      try { packageManager.getApplicationLabel(packageManager.getApplicationInfo(pkg, 0)).toString() } catch (e: Exception) { pkg }
    }
    /// Let an app through (its trackers no longer refused), or refuse them again.
    @JavascriptInterface
    fun allow(pkg: String, through: Boolean) {
      val prefs = getSharedPreferences("vpn", MODE_PRIVATE)
      val set = prefs.getStringSet("allowed", emptySet())!!.toMutableSet()
      if (through) set.add(pkg) else set.remove(pkg)
      prefs.edit().putStringSet("allowed", set).apply()
      runOnUiThread {
        if (TrackerVpn.on) {
          startService(Intent(this@MainActivity, TrackerVpn::class.java).setAction(TrackerVpn.STOP))
          wv?.postDelayed({ vpnWanted(true) }, 600)
        } else TrackerVpn.allowed = set
      }
    }
  }

  /// **The native player** (`PlaybackService`): the app hands it the queue and presses its keys; it
  /// says what it is doing (`fm-player`), twice a second while playing.
  inner class PlayerShell {
    private fun withPlayer(f: (androidx.media3.exoplayer.ExoPlayer) -> Unit) {
      runOnUiThread {
        val p = PlaybackService.player
        if (p != null) f(p)
        else {
          val first = synchronized(PlaybackService.pending) {
            PlaybackService.pending.add { q -> listen(q); f(q) }
            PlaybackService.pending.size == 1
          }
          if (first) startService(Intent(this@MainActivity, PlaybackService::class.java))
        }
      }
    }

    /// The queue (`[{path, title, artist, album, cover}]`), starting at `index`, at `seconds`.
    @JavascriptInterface
    fun setQueue(json: String, index: Int, seconds: Double, play: Boolean) = withPlayer { p ->
      val a = JSONArray(json)
      val items = (0 until a.length()).map { i ->
        val o = a.getJSONObject(i)
        PlaybackService.item(o.getString("path"), o.optString("title"), o.optString("artist"), o.optString("album"),
          o.optString("cover").takeIf { it.isNotEmpty() })
      }
      p.setMediaItems(items, index.coerceIn(0, (items.size - 1).coerceAtLeast(0)), (seconds * 1000).toLong())
      p.prepare()
      p.playWhenReady = play
      listen(p)
    }
    // A player left in its error state plays nothing until it is prepared again.
    @JavascriptInterface fun play() = withPlayer { if (it.playerError != null) it.prepare(); it.play() }
    @JavascriptInterface fun pause() = withPlayer { it.pause() }
    @JavascriptInterface fun next() = withPlayer { if (it.hasNextMediaItem()) it.seekToNextMediaItem() }
    @JavascriptInterface fun previous() = withPlayer { it.seekToPrevious() }
    @JavascriptInterface fun jump(index: Int) = withPlayer {
      it.seekTo(index, 0)
      if (it.playerError != null) it.prepare()
      it.play()
    }
    @JavascriptInterface fun seek(seconds: Double) = withPlayer { it.seekTo((seconds * 1000).toLong()) }
    @JavascriptInterface fun volume(v: Double) = withPlayer { it.volume = v.toFloat().coerceIn(0f, 1f) }
    @JavascriptInterface fun modes(shuffle: Boolean, repeat: String) = withPlayer {
      it.shuffleModeEnabled = shuffle
      it.repeatMode = when (repeat) { "all" -> Player.REPEAT_MODE_ALL; "one" -> Player.REPEAT_MODE_ONE; else -> Player.REPEAT_MODE_OFF }
    }
    @JavascriptInterface fun stop() = withPlayer { it.stop(); it.clearMediaItems() }
  }

  private var listening: androidx.media3.exoplayer.ExoPlayer? = null
  private val tick = object : Runnable {
    override fun run() {
      val p = PlaybackService.player ?: return
      tellPlayer(p)
      if (p.isPlaying) wv?.postDelayed(this, 500)
    }
  }
  private fun tellPlayer(p: androidx.media3.exoplayer.ExoPlayer) {
    val d = JSONObject().put("index", p.currentMediaItemIndex).put("playing", p.isPlaying)
      .put("time", p.currentPosition / 1000.0).put("length", if (p.duration > 0) p.duration / 1000.0 else 0.0)
      .put("ended", p.playbackState == Player.STATE_ENDED).put("error", p.playerError?.message ?: "")
      // How many songs it holds: none after Android stopped the service or the player was cleared,
      // and the page then hands the queue over again rather than asking for a song that is not there.
      .put("count", p.mediaItemCount)
    wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-player',{detail:$d}))", null)
  }
  private fun listen(p: androidx.media3.exoplayer.ExoPlayer) {
    if (listening === p) return
    listening = p
    p.addListener(object : Player.Listener {
      override fun onEvents(player: Player, events: Player.Events) {
        tellPlayer(p)
        wv?.removeCallbacks(tick)
        if (p.isPlaying) wv?.postDelayed(tick, 500)
      }
    })
  }

  /// **Save media** (`MediaRun`): the app asks, polls, and files the result through the core.
  inner class MediaShell {
    @JavascriptInterface fun workDir(): String = MediaRun.workDir(applicationContext).absolutePath
    @JavascriptInterface fun probe(id: String, url: String) = MediaRun.probe(applicationContext, id, url)
    @JavascriptInterface fun start(id: String, url: String, args: String) = MediaRun.start(applicationContext, id, url, args)
    @JavascriptInterface fun jobs(): String = MediaRun.jobs()
    @JavascriptInterface fun cancel(id: String) = MediaRun.cancel(id)
    @JavascriptInterface fun forget(id: String) = MediaRun.forget(id)
    @JavascriptInterface fun freshen() { Thread { MediaRun.freshen(applicationContext, true) }.start() }
  }

  inner class Pages {
    /// The pages' lists are read (until then every request goes through): the first start fetches
    /// them, so protection begins a little after it (`phone-media-check` waits on this).
    @JavascriptInterface
    fun blockReady(): Boolean = try { Blocker.ready() } catch (e: Throwable) { false }

    /// The engine's decision for one request (`""` through, `"-"` refused, else a stand-in's type):
    /// what a "why was this refused" view and the phone check ask.
    @JavascriptInterface
    fun blockVerdict(url: String, page: String, kind: String): String =
      try { Blocker.verdict(url, page, kind).substringBefore('\n') } catch (e: Throwable) { "error: $e" }

    /// What the blocking has now (`Blocker.state`): the engine, its switches, each list's size.
    @JavascriptInterface
    fun blockState(): String = try {
      val o = JSONObject(Blocker.state())
      val c = JSONObject()
      for ((k, v) in counts) c.put(k, v.blocked)
      o.put("counts", c).toString()
    } catch (e: Throwable) { "{}" }

    /// Draw the item's page over that box (the app's CSS pixels), opening it at `url` the first time.
    @JavascriptInterface
    fun show(id: String, url: String, x: Double, y: Double, w: Double, h: Double) {
      runOnUiThread {
        val page = pageFor(id, url)
        val d = resources.displayMetrics.density
        val main = wv
        val origin = IntArray(2).also { main?.getLocationInWindow(it) }
        val root = IntArray(2).also { (page.parent as? View)?.getLocationInWindow(it) }
        val lp = FrameLayout.LayoutParams((w * d).toInt(), (h * d).toInt())
        lp.leftMargin = (x * d).toInt() + origin[0] - root[0]
        lp.topMargin = (y * d).toInt() + origin[1] - root[1]
        page.layoutParams = lp
        page.visibility = View.VISIBLE
        page.bringToFront()
        lastShown = id
      }
    }

    @JavascriptInterface
    fun hide(id: String) {
      runOnUiThread { pages[id]?.visibility = View.GONE }
    }

    /// The item was closed: its page goes for good.
    @JavascriptInterface
    fun close(id: String) {
      runOnUiThread {
        val page = pages.remove(id) ?: return@runOnUiThread
        scripts.remove(id)
        counts.remove(id)
        seenMedia.remove(id)
        pageUrls.remove(id)
        (page.parent as? ViewGroup)?.removeView(page)
        page.destroy()
      }
    }

    /// **Save this page** (≡): its HTML handed to the app, which reads it as the computer's *Save page*
    /// does and keeps it as a note (`lib/web/read.ts`).
    @JavascriptInterface
    fun html(id: String) {
      runOnUiThread {
        val page = pages[id] ?: return@runOnUiThread
        page.evaluateJavascript("document.documentElement.outerHTML") { got ->
          try {
            val html = (JSONTokener(got).nextValue() as? String ?: return@evaluateJavascript).take(MAX_PAGE)
            val d = JSONObject().put("id", id).put("url", page.url ?: "").put("title", page.title ?: "")
              .put("html", html).put("seen", JSONArray(seenMedia[id]?.toList() ?: emptyList<String>()))
            wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-html',{detail:$d}))", null)
          } catch (e: Exception) { }
        }
      }
    }

    /// **This device's privacy switches** (`privacy.svelte.ts`): taken, given to every open page, and
    /// the fire button acted on when it was pressed.
    @JavascriptInterface
    fun privacy(json: String) {
      val fire = try { PagePrivacy.update(json) } catch (e: Exception) { false }
      // The phone-wide block asks the same engine, with the trackers' list.
      try {
        Blocker.configure(PagePrivacy.trackers, PagePrivacy.ads, PagePrivacy.cleanLinks)
        Blocker.configureApps(PagePrivacy.appTrackers, PagePrivacy.around)
      } catch (e: Throwable) { }
      runOnUiThread { try { vpnWanted(PagePrivacy.appTrackers) } catch (e: Throwable) { } }
      runOnUiThread {
        pages.forEach { (id, p) -> giveScripts(id, p) }
        if (fire) forget()
      }
    }

    /// A script the pages run at their start, handed over by the app (`dist/page/<name>.js`).
    @JavascriptInterface
    fun pageScript(name: String, text: String) {
      if (name == "consent") PagePrivacy.consentScript = text
      if (name == "procedural") PagePrivacy.proceduralScript = text
      if (name == "fingerprint") PagePrivacy.fingerprintScript = text
      runOnUiThread { pages.forEach { (id, p) -> giveScripts(id, p) } }
    }

    /// This build can block trackers in other apps (the F-Droid build's VPN).
    @JavascriptInterface
    fun vpn(): Boolean = BuildFlags.VPN

    /// The address row: a new address (or a search) in this page, made private as any other.
    @JavascriptInterface
    fun go(id: String, url: String) {
      runOnUiThread { pages[id]?.let { open(id, it, url) } }
    }

    @JavascriptInterface
    fun reload(id: String) {
      runOnUiThread { pages[id]?.reload() }
    }

    @JavascriptInterface
    fun back(id: String) {
      runOnUiThread { pages[id]?.let { if (it.canGoBack()) it.goBack() } }
    }

    /// **Bring the selection in** (`decisions.md` 2026-11-16, *a page is a layer*): the page's
    /// selection (its HTML and words), or the page itself when nothing is selected, with the page's
    /// exact address and title, into `note` (or the note in front, decided by the app).
    @JavascriptInterface
    fun bring(id: String, note: String) {
      runOnUiThread {
        val page = pages[id] ?: return@runOnUiThread
        page.evaluateJavascript(PICK) { got ->
          try {
            val json = JSONTokener(got).nextValue() as? String ?: return@evaluateJavascript
            val piece = JSONObject(json)
            if (piece.optString("text").length > MAX_TEXT) piece.put("text", piece.optString("text").take(MAX_TEXT))
            if (piece.optString("html").length > MAX_TEXT) piece.remove("html")
            piece.put("note", note)
            bring(piece)
          } catch (e: Exception) { }
        }
      }
    }
  }

  override fun onWebViewCreate(webView: WebView) {
    wv = webView
    webView.addJavascriptInterface(Insets(), "__fmInsets")
    // The update bridge, re-injected into every document like the insets one. See UpdateDownload.
    webView.addJavascriptInterface(UpdateDownload(), "__fmUpdate")
    // What another app shared: the page asks for it. See `takeShared`.
    webView.addJavascriptInterface(Share(), "__fmShare")
    // Web pages as items of the collection, drawn where the app says. See `Pages`.
    webView.addJavascriptInterface(Pages(), "__fmPage")
    // Trackers in other apps: their counts and the apps let through. See `VpnShell`.
    webView.addJavascriptInterface(VpnShell(), "__fmVpn")
    // The native player, playing on in the background. See `PlayerShell`.
    webView.addJavascriptInterface(PlayerShell(), "__fmPlayer")
    // Save media's runner (yt-dlp inside the app). See `MediaShell`.
    webView.addJavascriptInterface(MediaShell(), "__fmMedia")
    // A watch's files: the granted folder, and workouts out. See `WatchShell`.
    webView.addJavascriptInterface(WatchShell(), "__fmWatch")
    // **Back goes back in the page first**, as in any browser: while a page item is shown and has
    // somewhere to go back to. Otherwise Back is the app's (wry's callback, registered before this
    // one, so this one is asked first).
    onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
      override fun handleOnBackPressed() {
        val shown = pages.filter { it.value.visibility == View.VISIBLE }
        val page = shown[lastShown] ?: shown.values.firstOrNull()
        if (page != null && page.canGoBack()) {
          page.goBack()
          return
        }
        // A page with nowhere further back: the app leaves it (back to the tab it came from), as a
        // browser does, rather than Android leaving formicaria.
        val id = pages.entries.firstOrNull { it.value === page }?.key
        if (id != null) {
          val d = JSONObject().put("id", id)
          wv?.evaluateJavascript("window.dispatchEvent(new CustomEvent('fm-page-back',{detail:$d}))", null)
          return
        }
        isEnabled = false
        onBackPressedDispatcher.onBackPressed()
        isEnabled = true
      }
    })
    current = java.lang.ref.WeakReference(this)

    // **Applied before the page's own CSS**, on every navigation, so there is no frame in which
    // the app is laid out against a zero inset. Feature-gated exactly as `RustWebView.kt` gates
    // its own init scripts; without it the interface alone would still leave a flash.
    if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
      WebViewCompat.addDocumentStartJavaScript(webView, APPLY_INSETS, setOf("*"))
    }

    // Ask immediately rather than waiting for the system's own first dispatch: the sooner the real
    // numbers replace `fallbackTop()`, the smaller the window in which the page is laid out against
    // an approximation — even a good one.
    ViewCompat.requestApplyInsets(webView)

    ViewCompat.setOnApplyWindowInsetsListener(webView) { v, insets ->
      val i = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      val d = v.resources.displayMetrics.density
      top = i.top / d; right = i.right / d; bottom = i.bottom / d; left = i.left / d
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
      keyboard = maxOf(0, ime.bottom - i.bottom) / d
      // Now only the *update* channel — a rotation, the keyboard, a returned-to app. By the time
      // any of those happen the page exists, which is the one job this call can actually do.
      v.post { (v as WebView).evaluateJavascript(APPLY_INSETS, null) }
      insets
    }
  }

  // A rotation or coming back to the app can change the insets; asking again is cheap. At launch
  // `wv` is still null here — the webview arrives on wry's main-pipe message, after onResume has
  // returned — which is precisely why this cannot be the mechanism that gets the first values in.
  override fun onResume() {
    super.onResume()
    wv?.let { ViewCompat.requestApplyInsets(it) }
    // The watch's folder, on opening and on every return to the app (off the main thread).
    lookWatchFolder()
  }

  companion object {
    /// **What was brought in from a page** (`Pages.bring`), waiting for the app to take it: the
    /// selection's HTML and words, or the page itself, with the page's exact address and title, and
    /// the note it goes into.
    val brought = mutableListOf<JSONObject>()
    var current: java.lang.ref.WeakReference<MainActivity>? = null

    fun bring(piece: JSONObject) {
      synchronized(brought) { brought.add(piece) }
      val a = current?.get() ?: return
      a.runOnUiThread { a.wv?.evaluateJavascript("window.dispatchEvent(new Event('fm-share'))", null) }
    }

    /// The most one shared file may be: it travels to the page as base64 text.
    private const val MAX_SHARE = 25L * 1024 * 1024
    /// A shared file that is neither a picture nor a watch's activity file.
    private const val NOT_TAKEN = "not a picture or a watch's activity file (.fit)"
    /// How a `.fit` taken from the share menu is labelled for the page.
    private const val FIT_MIME = "application/vnd.ant.fit"
    /// The second look at the watch's folder, for files still being written.
    private const val WATCH_SETTLE_MS = 3000L
    /// The most files one look at the watch's folder lists, and how many it remembers as taken.
    private const val MAX_WATCH_LISTED = 5000
    private const val MAX_WATCH_SEEN = 20000
    /// Gadgetbridge's packages: the release, then its two nightly builds.
    private val GB_PACKAGES = listOf(
      "nodomain.freeyourgadget.gadgetbridge",
      "nodomain.freeyourgadget.gadgetbridge.nightly",
      "nodomain.freeyourgadget.gadgetbridge.nightly_nopebble",
    )
    /// Gadgetbridge's Intent API: the sync trigger it takes.
    private const val GB_SYNC = "nodomain.freeyourgadget.gadgetbridge.command.ACTIVITY_SYNC"
    private const val GB_STORE = "https://f-droid.org/packages/nodomain.freeyourgadget.gadgetbridge/"
    /// After *Sync now*, the folder's looks while Gadgetbridge syncs and exports.
    private val WATCH_SYNC_LOOKS = listOf(30_000L, 90_000L, 180_000L)
    /// The most of a page's HTML handed over to be saved (characters).
    private const val MAX_PAGE = 5_000_000
    /// The selection bar's *Bring into note*.
    private const val BRING_ITEM = 0x666d
    /// What a page gives when brought in: its selection, or itself.
    private const val PICK = """
      (function () {
        var s = window.getSelection(), t = String(s || '').trim(), h = '';
        if (t && s.rangeCount) {
          var d = document.createElement('div');
          d.appendChild(s.getRangeAt(0).cloneContents());
          h = d.innerHTML;
        }
        return JSON.stringify({ kind: t ? 'selection' : 'page', text: t, html: h,
          pageUrl: location.href, pageTitle: document.title });
      })()
    """
    /// A passage brought in is text a person chose; a whole book pasted into Share is cut here.
    private const val MAX_TEXT = 100_000
    /// The newest release's APK — the address `docs/src/user/android.md` gives for the first install.
    private const val LATEST_APK =
      "https://github.com/formicaria-org/formicaria/releases/latest/download/formicaria-android.apk"

    /// Reads the interface and writes the four properties `app.css` consumes. Defensive because it
    /// also runs at document start, where a hostile-looking absence is simply "not injected yet".
    private const val APPLY_INSETS = """
      (function () {
        try {
          var i = JSON.parse(window.__fmInsets.json());
          var s = document.documentElement.style;
          s.setProperty('--safe-top', i.top + 'px');
          s.setProperty('--safe-right', i.right + 'px');
          s.setProperty('--safe-bottom', i.bottom + 'px');
          s.setProperty('--safe-left', i.left + 'px');
          // The keyboard: written only when it changes, so a rotation does not re-fire the event.
          var kb = (i.keyboard || 0) + 'px';
          if (s.getPropertyValue('--kb') !== kb) {
            s.setProperty('--kb', kb);
            if (i.keyboard > 0) document.documentElement.setAttribute('data-keyboard', '');
            else document.documentElement.removeAttribute('data-keyboard');
            window.dispatchEvent(new Event('fm-keyboard'));
          }
        } catch (e) {}
      })();
    """
  }
}
