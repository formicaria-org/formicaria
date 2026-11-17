package dev.formicaria.notes

import android.content.Context
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import android.media.MediaMuxer
import com.yausername.youtubedl_android.YoutubeDL
import com.yausername.youtubedl_android.YoutubeDLRequest
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import java.util.concurrent.ConcurrentHashMap

class MediaJob(val id: String) {
  @Volatile var state = "tools"
  @Volatile var progress = -1.0
  @Volatile var title = ""
  @Volatile var file = ""
  @Volatile var info = "null"
  @Volatile var probe = ""
  @Volatile var error = ""
  fun json(): JSONObject = JSONObject().put("id", id).put("state", state)
    .put("progress", if (progress < 0) JSONObject.NULL else progress).put("title", title).put("file", file)
    .put("info", info).put("probe", probe).put("error", error)
}

object MediaRun {
  private val jobs = ConcurrentHashMap<String, MediaJob>()
  @Volatile private var ready = false

  /** The tools unpacked (once, the first time) and yt-dlp kept current (weekly, by its own updater). */
  private fun ensure(ctx: Context) {
    synchronized(this) {
      if (!ready) {
        // yt-dlp and Python only: no ffmpeg in the app (2026-11-17). The core makes MP3 and writes the
        // tags (`fm-audio`); a video's two streams are joined below, by Android's own muxer.
        YoutubeDL.getInstance().init(ctx)
        ready = true
      }
    }
    freshen(ctx, false)
  }

  fun freshen(ctx: Context, force: Boolean) {
    val prefs = ctx.getSharedPreferences("media", Context.MODE_PRIVATE)
    val last = prefs.getLong("checked", 0L)
    if (!force && System.currentTimeMillis() - last < 7L * 24 * 3600 * 1000) return
    try { YoutubeDL.getInstance().updateYoutubeDL(ctx, YoutubeDL.UpdateChannel._STABLE) } catch (e: Throwable) { }
    prefs.edit().putLong("checked", System.currentTimeMillis()).apply()
  }

  fun workDir(ctx: Context): File = File(ctx.cacheDir, "media").apply { mkdirs() }

  /** yt-dlp's `-J` for an address: what it holds, before anything is fetched. */
  fun probe(ctx: Context, id: String, url: String) {
    val job = MediaJob(id).also { jobs[id] = it }
    Thread {
      try {
        ensure(ctx)
        job.state = "probing"
        val r = YoutubeDLRequest(url)
        r.addOption("-J"); r.addOption("--no-playlist"); r.addOption("--no-warnings")
        job.probe = YoutubeDL.getInstance().execute(r, id, null).out
        job.state = "probed"
      } catch (e: Throwable) {
        job.error = "${e.javaClass.simpleName}: ${e.message ?: "the media could not be looked at"}"
        job.state = "failed"
      }
    }.start()
  }

  /** yt-dlp's arguments (the core's run, the address last) as a request. */
  private fun request(url: String, tokens: List<String>): YoutubeDLRequest {
    val r = YoutubeDLRequest(url)
    var i = 0
    while (i < tokens.size) {
      val t = tokens[i]
      if (t == "--") break
      val next = tokens.getOrNull(i + 1)
      if (next != null && !next.startsWith("-")) { r.addOption(t, next); i += 2 } else { r.addOption(t); i += 1 }
    }
    return r
  }

  private fun runOnce(job: MediaJob, id: String, url: String, run: JSONArray): File {
    val tokens = (0 until run.length()).map { run.getString(it) }
    job.file = ""
    val resp = YoutubeDL.getInstance().execute(request(url, tokens), id) { _, _, line -> read(job, line) }
    resp.out.lines().forEach { read(job, it) }
    if (job.state == "cancelled") throw InterruptedException("cancelled")
    val f = File(job.file)
    if (job.file.isEmpty() || !f.exists()) throw IllegalStateException("yt-dlp finished without a file")
    return f
  }

  /** Fetch for a choice: `plan` is the core's (`fm_app::media::phone_runs`): the runs, and for a
   *  video the sound to fetch by the picture's container, joined afterwards only if the picture has
   *  no sound of its own. */
  fun start(ctx: Context, id: String, url: String, plan: String) {
    val job = MediaJob(id).also { jobs[id] = it }
    Thread {
      try {
        ensure(ctx)
        job.state = "downloading"
        val p = JSONObject(plan)
        var file = runOnce(job, "$id-0", url, p.getJSONArray("runs").getJSONArray(0))
        if (p.optBoolean("join") && !hasSound(file)) {
          val kind = if (file.extension.equals("webm", true)) "webm" else "mp4"
          val sound = runOnce(job, "$id-1", url, p.getJSONObject("audio").getJSONArray(kind))
          job.state = "converting"
          job.progress = -1.0
          file = join(file, sound, kind)
        }
        job.file = file.absolutePath
        job.state = "fetched"
      } catch (e: Throwable) {
        if (job.state != "cancelled") {
          // What failed, by name, then what yt-dlp last said (`media::explain` finds its ERROR line,
          // or shows the first thing said).
          val tail = said[id]?.let { q -> synchronized(q) { q.joinToString("\n") } } ?: ""
          job.error = listOf("${e.javaClass.simpleName}: ${e.message ?: ""}", tail).filter { it.isNotBlank() }.joinToString("\n")
          job.state = "failed"
        }
      } finally {
        said.remove(id)
      }
    }.start()
  }

  private fun hasSound(f: File): Boolean {
    val ex = MediaExtractor()
    return try {
      ex.setDataSource(f.path)
      (0 until ex.trackCount).any { (ex.getTrackFormat(it).getString(MediaFormat.KEY_MIME) ?: "").startsWith("audio/") }
    } catch (e: Exception) { false } finally { ex.release() }
  }

  /** **A video's picture and sound, joined** into one file without re-encoding, by Android's own
   *  muxer: MP4 (H.264 with AAC) or WebM (VP8/VP9 with Opus or Vorbis). The parts are removed. */
  private fun join(video: File, audio: File, kind: String): File {
    val out = File(video.parentFile, "media.$kind")
    val v = MediaExtractor().apply { setDataSource(video.path) }
    val a = MediaExtractor().apply { setDataSource(audio.path) }
    val mime = { ex: MediaExtractor, i: Int -> ex.getTrackFormat(i).getString(MediaFormat.KEY_MIME) ?: "" }
    val format = if (kind == "webm") MediaMuxer.OutputFormat.MUXER_OUTPUT_WEBM else MediaMuxer.OutputFormat.MUXER_OUTPUT_MPEG_4
    val mux = MediaMuxer(out.path, format)
    val pairs = listOf(v to "video/", a to "audio/").map { (ex, k) ->
      val t = (0 until ex.trackCount).first { mime(ex, it).startsWith(k) }
      ex.selectTrack(t)
      ex to mux.addTrack(ex.getTrackFormat(t))
    }
    mux.start()
    val buf = java.nio.ByteBuffer.allocate(4 * 1024 * 1024)
    val info = MediaCodec.BufferInfo()
    for ((ex, track) in pairs) {
      while (true) {
        val n = ex.readSampleData(buf, 0)
        if (n < 0) break
        info.offset = 0
        info.size = n
        info.presentationTimeUs = ex.sampleTime
        info.flags = if (ex.sampleFlags and MediaExtractor.SAMPLE_FLAG_SYNC != 0) MediaCodec.BUFFER_FLAG_KEY_FRAME else 0
        mux.writeSampleData(track, buf, info)
        ex.advance()
      }
    }
    mux.stop()
    mux.release()
    v.release()
    a.release()
    video.delete()
    audio.delete()
    return out
  }

  /** The last lines yt-dlp said, kept so a failure can say more than its last line. */
  private val said = ConcurrentHashMap<String, java.util.ArrayDeque<String>>()

  private fun read(job: MediaJob, line: String) {
    val l = line.trim()
    if (l.isNotEmpty() && !l.startsWith("FMPROG ")) {
      val q = said.getOrPut(job.id) { java.util.ArrayDeque() }
      synchronized(q) { q.addLast(l); while (q.size > 12) q.removeFirst() }
    }
    when {
      l.startsWith("FMPROG ") -> {
        val n = l.removePrefix("FMPROG ").split(" ").map { it.toDoubleOrNull() }
        val done = n.getOrNull(0); val total = n.getOrNull(1) ?: n.getOrNull(2)
        if (done != null && total != null && total > 0) job.progress = (done / total).coerceIn(0.0, 1.0)
      }
      l.startsWith("FMFILE ") -> job.file = l.removePrefix("FMFILE ")
      l.startsWith("FMINFO ") -> { job.info = l.removePrefix("FMINFO "); job.title = try { JSONObject(job.info).optString("title") } catch (e: Exception) { "" } }
      l.startsWith("[ExtractAudio]") || l.startsWith("[Merger]") -> { job.state = "converting"; job.progress = -1.0 }
    }
  }

  fun cancel(id: String) {
    jobs[id]?.state = "cancelled"
    try { YoutubeDL.getInstance().destroyProcessById(id) } catch (e: Throwable) { }
  }

  fun jobs(): String = JSONArray(jobs.values.map { it.json() }).toString()

  /** A job done with (filed, or its failure shown): its leftovers go. */
  fun forget(id: String) {
    val j = jobs.remove(id) ?: return
    if (j.file.isNotEmpty()) File(j.file).delete()
  }
}
