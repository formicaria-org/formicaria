# Maps in fm, and what a watch's GPS track could give (research, 2026-11-21)

> **Research, not a ruling.** No decision is made here; building anything below needs the entries
> listed in §7 first. Sources were read on 2026-11-21 by two research passes; claims marked
> **UNVERIFIED** could not be confirmed at source.

## 0. Where fm stands

- A watch's FIT file is kept whole beside its entry, GPS track included, but the importer reads the
  per-second stream only as **counts** (`crates/fm-core/src/fit.rs`, `Stream`: "what it has, never
  the values"). Nothing draws a map, and nothing computes splits by distance, grade-adjusted pace,
  climbs or best efforts from positions. `training-tools-research-2026-11-20.md` §5.4 and §14 list
  some of these as candidates; none is built.
- **A watch of the Forerunner 165 class records in "Smart recording" by default; "Every Second" is an
  option** (Garmin's FR165 manual, June 2025 edition,
  https://www8.garmin.com/manuals/webhelp/GUID-607F08F6-33FC-40BF-9727-84E54043D82D/EN-US/Forerunner_165_Series_OM_EN-US.pdf).
  Anything computed from records must handle uneven gaps, not assume 1 Hz.
- fm's CSP blocks outside hosts, and nothing is sent anywhere unasked: a map that fetches tiles from a
  server is a new outbound request and a decision (§7).

## 1. What watch platforms show, and what is reproducible

| Shown | Reproducible from the track? | Basis |
|---|---|---|
| Map, distance, splits, laps, pace | **Yes** | Plain computation |
| Elevation gain/loss, profile | **Yes**, with care (§3) | Plain computation |
| Climbs with categories | **Yes** | Garmin: ≥ 500 m at ≥ 3 % average; score = grade × length; Cat 4 > 8,000 … HC > 80,000 ([Garmin](https://www8.garmin.com/manuals-apac/webhelp/edge540/EN-SG/GUID-A8C70AB7-0F46-4FF4-97CF-E3862CE8909A-89.html)); Strava the same ([Strava](https://support.strava.com/en-us/articles/15402015-how-does-strava-categorize-climbs)) |
| Best efforts (400 m … 50 km) | **Yes** | Sliding window ([Strava](https://support.strava.com/en-us/articles/15401661-how-do-best-efforts-work-for-running-on-strava)) |
| Grade-adjusted pace | **Yes, Minetti's** | Strava's own model is patented (§3) |
| Route/segment matching, own heatmap | **Yes** | Leaderboards need a crowd |
| Training Effect, EPOC, VO2max, Training Status | **No** | Firstbeat's models (neural networks over HR/HRV, [Firstbeat](https://firstbeat.com/en/science-and-physiology/epoc-and-training-effect)); fm keeps the watch's own values as `watch_*` |

## 2. A map stack that fits fm

**Renderer.**
- **MapLibre GL JS 6.19.0**, BSD-3. Vector and raster tiles, hillshade, heatmap and colour-relief
  layers. About 300 KB gzipped (bundlephobia, not primary).
  - **WebGL2 is required since v6**, and since 6.7 the constructor throws `GPUInitializationError`
    without it ([CHANGELOG](https://raw.githubusercontent.com/maplibre/maplibre-gl-js/main/CHANGELOG.md)).
    Android System WebView has WebGL2 on current devices per caniuse; that every device's WebView
    gets it is **UNVERIFIED** (GPU blocklists), so a fallback is needed.
  - Under a strict CSP it needs `worker-src 'self'`, `img-src data: blob: 'self'`, and
    `setWorkerUrl` with a same-origin worker ([docs](https://maplibre.org/maplibre-gl-js/docs/)).
- **Alternatives:**
  - Leaflet 1.9.4: BSD-2, about 40 KB, raster only.
  - OpenLayers 10.11: BSD-2, vector tiles, about 86 KB gzipped.
  - MapLibre Native Android: BSD-2, only if the map leaves the WebView.

**Tiles: where they come from, and what that costs in privacy.**
- **A tile's address is its place.** `{z}/{x}/{y}` converts straight to latitude/longitude, and a
  z17 tile is about 306 m wide ([OSM wiki](https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames)).
  Every tile request tells the server where the person is looking, with their IP and User-Agent.
- **tile.openstreetmap.org:** interactive viewing only, a stable identifying User-Agent, attribution,
  and honoured caching. **No bulk download, no prefetch: "Offline use is not permitted"**
  ([policy](https://operations.osmfoundation.org/policies/tiles/); the vector service is the same,
  [policy](https://operations.osmfoundation.org/policies/vector/)).
- **OpenFreeMap:** no keys, no view limits, donation-funded, no SLA, possibly behind Cloudflare.
  Offline downloads are capped at 3,000 tiles or 50 MB. Error logs can hold IP plus URL for 7 days.
  It offers weekly planet files for self-hosting ([site](https://openfreemap.org/),
  [ToS](https://openfreemap.org/tos/), [privacy](https://openfreemap.org/privacy/)).
- **Protomaps / PMTiles:** one read-only file, read by byte ranges ([docs](https://docs.protomaps.com/pmtiles/)).
  - The basemap code is BSD-3, the style CC0, and the tiles are an ODbL Produced Work.
  - The planet (z0–15) is about 120–139 GB, built daily.
  - A region is cut with `pmtiles extract --bbox=… [--maxzoom]`, and `--dry-run` gives the size
    first ([CLI](https://docs.protomaps.com/pmtiles/cli)).
  - Measured sizes: Switzerland about 180 MB and Berlin about 71 MB at z15; the US is 4.3 GB at
    z13 and 19 GB at z15. Each zoom level roughly doubles the size
    ([go-pmtiles#68](https://github.com/protomaps/go-pmtiles/issues/68), [downloads](https://docs.protomaps.com/basemaps/downloads)).
- **Commercial free tiers need keys and limit use:**
  - MapTiler Free: personal/non-commercial ([pricing](https://www.maptiler.com/cloud/pricing/)).
  - Stadia Free: no commercial use ([pricing](https://stadiamaps.com/pricing/)).
  - Thunderforest Hobby: offline only on paid plans ([pricing](https://www.thunderforest.com/pricing/)).
- **Attribution (ODbL):** "© OpenStreetMap contributors", visible or one tap away
  ([guidelines](https://osmfoundation.org/wiki/Licence/Attribution_Guidelines)).

**Reading a local PMTiles file in the app.**
- pmtiles JS 4.5 (BSD-3) takes any custom `Source` (`getBytes(offset, length)`)
  ([source](https://raw.githubusercontent.com/protomaps/PMTiles/main/js/src/index.ts)).
- **Range requests to Tauri's asset protocol fail on Android (`net::ERR_FAILED`)**; people use a
  localhost server or native code ([tauri#12243](https://github.com/orgs/tauri-apps/discussions/12243)).
- So in fm a `Source` that reads bytes through the Rust core (IPC on the phone, a range-capable
  route on the computer) avoids it. That is an inference, untested.

**Elevation and terrain.**
- **Copernicus GLO-30**, 1°×1° COG tiles of about 30–45 MB each, measured
  ([S3](https://copernicus-dem-30m.s3.amazonaws.com/readme.html)).
  - It is a *surface* model: buildings and tree canopy are included.
  - Free reuse with a required notice and a liability disclaimer passed on to users
    ([licence PDF](https://dataspace.copernicus.eu/sites/default/files/media/files/2025-06/copernicus_contributing_mission_data_access_v2_cop_dem_licenses.pdf), pp. 22–23).
  - Some tiles are not released.
- **SRTM:** credit requested.
- **AWS Terrain Tiles:** mixed sources, each with its own attribution
  ([attribution](https://github.com/tilezen/joerd/blob/master/docs/attribution.md)).
- **Mapterhorn:** BSD-3 code, Terrarium WebP to z17 ([data access](https://mapterhorn.com/data-access/));
  endpoint terms **UNVERIFIED**.
- MapLibre's `raster-dem` takes terrarium encoding and can be served from PMTiles
  ([spec](https://maplibre.org/maplibre-style-spec/sources/)).

**Routing (later).**

| Engine | Licence | Offline on Android | Uses elevation |
|---|---|---|---|
| **BRouter** | MIT | Yes (own app; engine for OsmAnd/Locus) | Yes (SRTM). Profiles: hiking, trekking, gravel, MTB. Data in 5°×5° files of 34–254 MB, weekly ([repo](https://github.com/abrensch/brouter), [segments](https://brouter.de/brouter/segments4/)) |
| Valhalla 3.9 | MIT | Yes (valhalla-mobile) | Yes; also has map matching (Meili) ([docs](https://valhalla.github.io/valhalla/concepts/costing/elevation-costing/)) |
| GraphHopper 11 | Apache-2.0 | Not officially supported now | Yes (srtm, pmtiles, …) ([elevation](https://github.com/graphhopper/graphhopper/blob/master/docs/core/elevation.md)) |
| OSRM | BSD-2 | Not stated | Only via Lua raster ([profiles](https://raw.githubusercontent.com/Project-OSRM/osrm-backend/master/docs/profiles.md)) |

## 3. The methods, and what the science says

- **GPS distance:**
  - Wrist GNSS distance error is 3.2–6.1 % MAPE across 8 watches, worst in town and forest,
    underestimating by up to 9 % (Gilgen-Ammann 2020, [doi:10.2196/17118](https://pubmed.ncbi.nlm.nih.gov/32396865/)).
  - Position noise makes summed point distances **too long**, by an amount that depends on how
    correlated the errors are (Ranacher 2016, [arXiv:1504.04504](https://arxiv.org/abs/1504.04504)).
  - **Prefer the watch's own distance and speed fields.** Otherwise smooth first: a Kalman filter
    plus a Rauch–Tung–Striebel smoother (`adskalman`, MIT/Apache).
  - Simplify the line only for display: Ramer–Douglas–Peucker / Visvalingam, both in `geo`.
- **Elevation gain:**
  - Arm swing alone adds 4.0–7.4 m per 1,200 m on a flat track (Ammann 2016,
    [PMC5260550](https://pmc.ncbi.nlm.nih.gov/articles/PMC5260550)).
  - Barometric watches underestimate hills by 3.3–9.8 % at default settings
    ([doi:10.1007/s12283-016-0207-9](https://folia.unifr.ch/unifr/documents/305444)).
  - The best barometer-assisted receivers have a 3.6–4.1 m RMS error and a −3.8 % total-gain bias
    against a DEM (Szot 2023, doi:10.1007/s12283-023-00443-3).
  - Open tools use a **hysteresis**. GoldenCheetah's default is 3 m: on one ride, 3 m gave 475 m of
    gain and 1 m gave 532 m ([thread](https://groups.google.com/g/golden-cheetah-developers/c/uOyggV202qI)).
  - Tune the threshold on real files (locally, never committed).
  - Whether a watch of the FR165 class has a barometer is listed by retailers, not confirmed by
    Garmin: **UNVERIFIED**.
- **Grade-adjusted pace (Minetti 2002):**
  - The cost of running on a gradient, measured from −0.45 to +0.45:
    `Cr(i) = 155.4i⁵ − 30.4i⁴ − 43.3i³ + 46.3i² + 19.5i + 3.6` J·kg⁻¹·m⁻¹; GAP factor = Cr(i)/3.6
    ([doi:10.1152/japplphysiol.01177.2001](https://iris.unibs.it/retrieve/ddc633e4-3c03-4e2e-e053-3705fe0a4c80/Minetti%20JAP%202002.pdf)).
  - **Strava's HR-based model is patented**: US11623121B1, priority 2018, expiry around 2041
    ([patent](https://patents.google.com/patent/US11623121)).
  - Its 2017 blog post could not be read (403); what it says is from an excerpt, **UNVERIFIED**.
    Use Minetti.
- **Best efforts:** a sliding window over cumulative distance or time.
- **Route matching:** Fréchet (Alt & Godau 1995) and Hausdorff distance, both in `geo`.
- **Map matching:** HMM (Newson & Krumm 2009), as in Valhalla's Meili.
- **Privacy zones:**
  - Strava hides the first and last 200 m, or a radius up to 1 mile around an address
    ([Strava](https://support.strava.com/hc/en-us/articles/115000173384)).
  - Yet up to **85 %** of protected places were recovered from 1.4 M activities using distance
    metadata and the street grid (Dhondt, CCS 2022, doi:10.1145/3548606.3560616).
  - **Hide the distance and time inside the zone too, not only the drawn line.** In fm this matters
    only for an export or a share: nothing leaves the device otherwise.

## 4. Open tools and crates

- **Reference only (GPL/AGPL, never linked):** GoldenCheetah (GPL-3), FitTrackee (AGPL-3), Endurain
  (AGPL-3), GPXSee (GPL-3), GPSBabel (GPL-2, usable as a separate program), Gadgetbridge (AGPL-3).
  Runalyze's open version is discontinued.
- **Reusable (permissive):** OpenTracks (Apache-2.0), gpxpy (Apache-2.0), Valhalla (MIT), BRouter (MIT),
  GraphHopper (Apache-2.0), OSRM (BSD-2).
- **Rust crates:** `geo` 0.33 (MIT/Apache), `gpx` (MIT), `rustyfit` (BSD-3, already in fm),
  `srtm_reader` (BSD-3), `geographiclib-rs` (MIT), `adskalman` (MIT/Apache), `minikalman` (MIT),
  `coursepointer` (MIT, GPX → FIT course with course points, usable as a library).

## 5. Routes to the watch

- **Courses on a watch of the FR165 class:** its manual has *Navigation → Courses → Do Course*,
  reverse, map and elevation plot, and describes sending courses only from Garmin Connect (manual
  above, p. 50).
- **Course file:** File Id type 6, then Course, Lap(s), a timer-start event, Records (timestamp and
  distance required), optional Course Points, and a timer-stop event
  ([Garmin](https://developer.garmin.com/fit/articles/file-types/course.html)).
- **Without Connect:**
  - USB to `GARMIN/NewFiles` is documented on other Garmin models; not in the FR165 manual, so
    **UNVERIFIED** there.
  - Gadgetbridge lists "File upload: Courses (with navigation instructions)" for Garmin, depending on
    the device ([GB](https://gadgetbridge.org/basics/topics/garmin/)).
  - Both need a device test. This is the coach plan's E1/E-cal family, a new experiment E-course.

## 6. What fm could compute without any proprietary model (ranked)

1. Splits, laps, moving time and pace: the FIT distance/speed fields first, smoothed positions as the
   fallback; uneven record gaps handled.
2. Elevation gain/loss with hysteresis (start at 3 m and tune on real files); barometric altitude
   first, DEM as an optional cross-check.
3. Best efforts and mean-maximal pace curves.
4. Minetti GAP.
5. Climbs with the published categories.
6. Privacy zones on export/share, hiding distance and time as well as the line.
7. Repeat-route comparison (Fréchet/Hausdorff).
8. FIT courses out (rustyfit or coursepointer), via USB or Gadgetbridge, after experiment E-course.
9. A heatmap of one's own tracks, drawn locally.

A map is needed for 1, 5, 7 and 9 to *show*, not to *compute*. Items 1–6 need no map at all and no
network.

## 7. Recommended direction, and the decisions it needs

**Recommended (not decided):**
- MapLibre GL JS + pmtiles JS on both devices, with a fallback when WebGL2 is missing.
- **Offline Protomaps regions the person downloads on request**, read through the Rust core, with
  glyphs and sprites carried locally. No tile server by default; an online layer (OpenFreeMap) is an
  opt-in.
- Elevation from the FIT file for profiles. Hillshade, later, from GLO-30 or Mapterhorn terrarium
  tiles with their notices.
- BRouter for planned routes, later.
- **Order:** track analysis first (items 1–6, no map, no network), then the map, then routes and
  courses.

**Decision entries needed first** (house format, the four questions):
1. *A route is computed from the kept FIT file*: positions read and stored as numbers in the note
   (which keys), uneven gaps, the 3-layer rule (`watch_*` vs fm's own).
2. *A map in fm*: MapLibre + PMTiles as dependencies (licences, size), the CSP's `worker-src`/`blob:`,
   and the WebGL2 fallback.
3. *Map data is fetched once, on request, and read locally*: a new outbound download (which host, how
   it is verified), region sizes on a phone, ODbL attribution. **Or**, the reverse, an online tile
   layer, saying what a tile server learns.
4. *Elevation and routing data*: GLO-30's notice and disclaimer, BRouter segments, if and when.
5. *Notes with coordinates on the map* (places, photos, events), if wanted: which property carries
   a place.
