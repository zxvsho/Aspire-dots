# aspire-bg reference (doc-2: verbatim code)

## State and Ui structs
```rust
 718 | struct State {
 719 |     all: Vec<String>,
 720 |     all_loaded: bool,
 721 |     shown: Vec<String>,
 722 |     base_shown: Vec<String>,
 723 |     current: usize,
 724 |     thumbs: HashMap<String, gtk::gdk::Texture>,
 725 |     pictures: HashMap<String, Picture>,
 726 |     thumb_gen: u64,
 727 |     search_gen: u64,
 728 |     search_pending: bool,
 729 |     search_rx: Option<std::sync::mpsc::Receiver<SearchMsg>>,
 730 |     search_drainer_active: bool,
 731 |     sample_gen: u64,
 732 |     sample_pending: bool,
 733 |     sample_rx: Option<std::sync::mpsc::Receiver<SampleMsg>>,
 734 |     sample_drainer_active: bool,
 735 |     index_ready: bool,
 736 | }
 737 | 
 738 | enum SampleMsg {
 739 |     Sample { gen: u64, shown: Vec<String> },
 740 | }
 741 | 
 742 | enum SearchMsg {
 743 |     Result {
 744 |         gen: u64,
 745 |         all: Option<Vec<String>>,
 746 |         index: Option<Vec<(String, String)>>,
 747 |         shown: Vec<String>,
 748 |     },
 749 | }
 750 | 
 751 | #[cfg(feature = "wallhaven")]
 752 | #[derive(Clone)]
 753 | enum WhEvent {
 754 |     Results {
 755 |         gen: u64,
 756 |         page: u32,
 757 |         last_page: u32,
 758 |         items: Vec<wallhaven::WhItem>,
 759 |     },
 760 |     Thumb {
 761 |         gen: u64,
 762 |         id: String,
 763 |         width: i32,
 764 |         height: i32,
 765 |         stride: usize,
 766 |         bytes: Vec<u8>,
 767 |     },
 768 |     Error {
 769 |         gen: u64,
 770 |         msg: String,
 771 |     },
 772 |     DownloadDone {
 773 |         path: String,
 774 |     },
 775 |     DownloadError {
 776 |         msg: String,
 777 |     },
 778 | }
 779 | 
 780 | #[cfg(feature = "wallhaven")]
 781 | struct WhState {
 782 |     categories: String,
 783 |     sort: String,
 784 |     purity: String,
 785 |     page: u32,
 786 |     last_page: u32,
 787 |     items: Vec<wallhaven::WhItem>,
 788 |     seen: HashSet<String>,
 789 |     thumbs: HashMap<String, gtk::gdk::Texture>,
 790 |     pictures: HashMap<String, Picture>,
 791 |     gen: u64,
 792 |     pending: Arc<AtomicUsize>,
 793 |     tx: Option<std::sync::mpsc::Sender<WhEvent>>,
 794 |     rx: Option<std::sync::mpsc::Receiver<WhEvent>>,
 795 |     drainer_active: bool,
 796 |     search_src: Rc<RefCell<Option<glib::SourceId>>>,
 797 |     opened: bool,
 798 |     max_pages: u32,
 799 | }
 800 | 
```
## Config load/save
```rust
 103 | fn load_config() -> BgConfig {
 104 |     ensure_config();
 105 |     let mut cfg = BgConfig {
 106 |         folder: format!("{}/Imágenes/Fondos", home()),
 107 |         backend: "swaybg".to_string(),
 108 |         monitors: "All".to_string(),
 109 |         fill: "fill".to_string(),
 110 |         sort: "name".to_string(),
 111 |         sort_mode: "random".to_string(),
 112 |         subfolders: false,
 113 |         show_hidden: false,
 114 |         zen_mode: false,
 115 |         color: "#000000".to_string(),
 116 |         wallpaper: String::new(),
 117 |         post_command: String::new(),
 118 |         wallhaven: WallhavenConfig {
 119 |             api_key: String::new(),
 120 |             purity: "100".to_string(),
 121 |             categories: "111".to_string(),
 122 |             atleast: "1920x1080".to_string(),
 123 |         },
 124 |     };
 125 |     let Ok(text) = std::fs::read_to_string(ini_path()) else {
 126 |         return cfg;
 127 |     };
 128 |     let mut section = String::new();
 129 |     for line in text.lines() {
 130 |         let line = line.trim();
 131 |         if line.starts_with('[') && line.ends_with(']') {
 132 |             section = line[1..line.len() - 1].trim().to_lowercase();
 133 |             continue;
 134 |         }
 135 |         let Some((k, v)) = line.split_once('=') else {
 136 |             continue;
 137 |         };
 138 |         let k = k.trim();
 139 |         let v = v.trim();
 140 |         if section == "wallhaven" {
 141 |             match k {
 142 |                 "api_key" => cfg.wallhaven.api_key = v.to_string(),
 143 |                 "purity" => cfg.wallhaven.purity = v.to_string(),
 144 |                 "categories" => cfg.wallhaven.categories = v.to_string(),
 145 |                 "atleast" => cfg.wallhaven.atleast = v.to_string(),
 146 |                 _ => {}
 147 |             }
 148 |         } else {
 149 |             match k {
 150 |                 "folder" => cfg.folder = shellexpand_home(v),
 151 |                 "backend" => cfg.backend = v.to_string(),
 152 |                 "monitors" => cfg.monitors = v.to_string(),
 153 |                 "fill" => cfg.fill = v.to_string(),
 154 |                 "sort" => cfg.sort = v.to_string(),
 155 |                 "sort_mode" => cfg.sort_mode = v.to_string(),
 156 |                 "subfolders" => cfg.subfolders = v.eq_ignore_ascii_case("true"),
 157 |                 "show_hidden" => cfg.show_hidden = v.eq_ignore_ascii_case("true"),
 158 |                 "zen_mode" => cfg.zen_mode = v.eq_ignore_ascii_case("true"),
 159 |                 "color" => cfg.color = v.to_string(),
 160 |                 "wallpaper" => cfg.wallpaper = shellexpand_home(v),
 161 |                 "post_command" => cfg.post_command = v.to_string(),
 162 |                 _ => {}
 163 |             }
 164 |         }
 165 |     }
 166 |     cfg
 167 | }
 168 | 
 169 | fn shellexpand_home(s: &str) -> String {
 170 |     if let Some(rest) = s.strip_prefix("~/") {
 171 |         format!("{}/{rest}", home())
 172 |     } else {
 173 |         s.to_string()
 174 |     }
 175 | }
 176 | 
 177 | /// Reescribe las claves conocidas preservando el resto del ini.
 178 | fn save_config(cfg: &BgConfig) {
 179 |     let settings_keys = [
 180 |         "folder", "backend", "monitors", "fill", "sort", "sort_mode", "subfolders",
 181 |         "show_hidden", "zen_mode", "color", "wallpaper", "post_command",
 182 |     ];
 183 |     let settings_vals = [
 184 |         cfg.folder.clone(),
 185 |         cfg.backend.clone(),
 186 |         cfg.monitors.clone(),
 187 |         cfg.fill.clone(),
 188 |         cfg.sort.clone(),
 189 |         cfg.sort_mode.clone(),
 190 |         (if cfg.subfolders { "True" } else { "False" }).to_string(),
 191 |         (if cfg.show_hidden { "True" } else { "False" }).to_string(),
 192 |         (if cfg.zen_mode { "True" } else { "False" }).to_string(),
 193 |         cfg.color.clone(),
 194 |         cfg.wallpaper.clone(),
 195 |         cfg.post_command.clone(),
 196 |     ];
 197 |     let wh_keys = ["api_key", "purity", "categories", "atleast"];
 198 |     let wh_vals = [
 199 |         cfg.wallhaven.api_key.clone(),
 200 |         cfg.wallhaven.purity.clone(),
 201 |         cfg.wallhaven.categories.clone(),
 202 |         cfg.wallhaven.atleast.clone(),
 203 |     ];
 204 | 
 205 |     let mut lines: Vec<String> = std::fs::read_to_string(ini_path())
 206 |         .map(|t| t.lines().map(|l| l.to_string()).collect())
 207 |         .unwrap_or_default();
 208 | 
 209 |     let mut seen_settings = vec![false; settings_keys.len()];
 210 |     let mut seen_wh = vec![false; wh_keys.len()];
 211 |     let mut section = String::new();
 212 |     let mut settings_header: Option<usize> = None;
 213 |     let mut wh_header: Option<usize> = None;
 214 | 
 215 |     for (i, line) in lines.iter_mut().enumerate() {
 216 |         let t = line.trim();
 217 |         if t.starts_with('[') && t.ends_with(']') {
 218 |             section = t[1..t.len() - 1].trim().to_lowercase();
 219 |             if section == "settings" {
 220 |                 settings_header = Some(i);
 221 |             } else if section == "wallhaven" {
 222 |                 wh_header = Some(i);
 223 |             }
 224 |             continue;
 225 |         }
 226 |         let Some((k, _)) = t.split_once('=') else {
 227 |             continue;
 228 |         };
 229 |         let k = k.trim();
 230 |         if section.is_empty() || section == "settings" {
 231 |             if let Some(idx) = settings_keys.iter().position(|x| *x == k) {
 232 |                 *line = format!("{} = {}", settings_keys[idx], settings_vals[idx]);
 233 |                 seen_settings[idx] = true;
 234 |             }
 235 |         } else if section == "wallhaven" {
 236 |             if let Some(idx) = wh_keys.iter().position(|x| *x == k) {
 237 |                 *line = format!("{} = {}", wh_keys[idx], wh_vals[idx]);
 238 |                 seen_wh[idx] = true;
 239 |             }
 240 |         }
 241 |     }
 242 | 
 243 |     // Asegura [Settings].
 244 |     if settings_header.is_none() {
 245 |         let mut block = vec!["[Settings]".to_string()];
 246 |         for (i, k) in settings_keys.iter().enumerate() {
 247 |             if !seen_settings[i] {
 248 |                 block.push(format!("{} = {}", k, settings_vals[i]));
 249 |                 seen_settings[i] = true;
 250 |             }
 251 |         }
 252 |         block.push(String::new());
 253 |         let mut new_lines = block;
 254 |         new_lines.extend(lines);
 255 |         lines = new_lines;
 256 |         // recalcula header
 257 |         settings_header = Some(0);
 258 |     }
 259 | 
 260 |     // Inserta settings faltantes tras el header.
```
## walk_images + sample_images
```rust
 321 | fn walk_images(folder: &str, recursive: bool, hidden: bool) -> Vec<String> {
 322 |     let mut v = Vec::new();
 323 |     let mut stack = vec![folder.to_string()];
 324 |     while let Some(dir) = stack.pop() {
 325 |         let Ok(rd) = std::fs::read_dir(&dir) else {
 326 |             continue;
 327 |         };
 328 |         for e in rd.flatten() {
 329 |             let p = e.path();
 330 |             let name = p
 331 |                 .file_name()
 332 |                 .map(|s| s.to_string_lossy().into_owned())
 333 |                 .unwrap_or_default();
 334 |             if !hidden && name.starts_with('.') {
 335 |                 continue;
 336 |             }
 337 |             if p.is_dir() {
 338 |                 if recursive {
 339 |                     stack.push(p.to_string_lossy().into_owned());
 340 |                 }
 341 |             } else if p.is_file() && is_image(&name) {
 342 |                 v.push(p.to_string_lossy().into_owned());
 343 |             }
 344 |         }
 345 |     }
 346 |     v
 347 | }
 348 | 
 349 | fn sample_images(folder: &str, recursive: bool, hidden: bool, sort_mode: &str, n: usize) -> Vec<String> {
 350 |     if n == 0 {
 351 |         return Vec::new();
 352 |     }
 353 |     if sort_mode == "random" {
 354 |         // Reservoir sampling: O(N), memoria O(n).
 355 |         let mut reservoir: Vec<String> = Vec::with_capacity(n);
 356 |         let mut count: u64 = 0;
 357 |         let mut seed = SystemTime::now()
 358 |             .duration_since(UNIX_EPOCH)
 359 |             .map(|d| d.subsec_nanos() as u64)
 360 |             .unwrap_or(0x9e3779b9)
 361 |             ^ (std::process::id() as u64);
 362 |         let mut rnd = || {
 363 |             seed ^= seed << 13;
 364 |             seed ^= seed >> 7;
 365 |             seed ^= seed << 17;
 366 |             seed
 367 |         };
 368 |         let mut stack = vec![folder.to_string()];
 369 |         while let Some(dir) = stack.pop() {
 370 |             let Ok(rd) = std::fs::read_dir(&dir) else { continue };
 371 |             for e in rd.flatten() {
 372 |                 let p = e.path();
 373 |                 let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
 374 |                 if !hidden && name.starts_with('.') {
 375 |                     continue;
 376 |                 }
 377 |                 if p.is_dir() {
 378 |                     if recursive {
 379 |                         stack.push(p.to_string_lossy().into_owned());
 380 |                     }
 381 |                 } else if p.is_file() && is_image(&name) {
 382 |                     count += 1;
 383 |                     if reservoir.len() < n {
 384 |                         reservoir.push(p.to_string_lossy().into_owned());
 385 |                     } else {
 386 |                         let j = (rnd() % count) as usize;
 387 |                         if j < n {
 388 |                             reservoir[j] = p.to_string_lossy().into_owned();
 389 |                         }
 390 |                     }
 391 |                 }
 392 |             }
 393 |         }
 394 |         let _ = count;
 395 |         reservoir
 396 |     } else {
 397 |         // Top-n por mtime/created con heap acotado (sin Vec completo).
 398 |         use std::cmp::Reverse;
 399 |         use std::collections::BinaryHeap;
 400 |         let mut heap: BinaryHeap<Reverse<(SystemTime, String)>> = BinaryHeap::new();
 401 |         let mut stack = vec![folder.to_string()];
 402 |         while let Some(dir) = stack.pop() {
 403 |             let Ok(rd) = std::fs::read_dir(&dir) else { continue };
 404 |             for e in rd.flatten() {
 405 |                 let p = e.path();
 406 |                 let name = p.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
 407 |                 if !hidden && name.starts_with('.') {
 408 |                     continue;
 409 |                 }
 410 |                 if p.is_dir() {
 411 |                     if recursive {
 412 |                         stack.push(p.to_string_lossy().into_owned());
 413 |                     }
 414 |                 } else if p.is_file() && is_image(&name) {
 415 |                     let key = std::fs::metadata(&p).ok().and_then(|m| {
 416 |                         if sort_mode == "created" {
 417 |                             m.created().ok().or_else(|| m.modified().ok())
 418 |                         } else {
 419 |                             m.modified().ok()
 420 |                         }
 421 |                     });
 422 |                     if let Some(k) = key {
 423 |                         heap.push(Reverse((k, p.to_string_lossy().into_owned())));
 424 |                         if heap.len() > n {
 425 |                             heap.pop();
 426 |                         }
 427 |                     }
 428 |                 }
 429 |             }
 430 |         }
 431 |         let mut v: Vec<_> = heap.into_sorted_vec();
 432 |         v.reverse();
 433 |         v.into_iter().map(|Reverse((_, p))| p).collect()
```
## reshuffle_20
```rust
1176 | fn reshuffle_20(ui: &Ui) {
1177 |     if ui.config.borrow().sort_mode != "random" {
1178 |         return;
1179 |     }
1180 |     let all = ui.state.borrow().all.clone();
1181 |     if all.is_empty() {
1182 |         // no index yet: just resample
1183 |         resample(ui);
1184 |         return;
1185 |     }
1186 |     let shown = pick_batch(&all, 20);
1187 |     {
1188 |         let mut st = ui.state.borrow_mut();
1189 |         st.base_shown = shown.clone();
1190 |         st.shown = shown;
1191 |         st.current = 0;
1192 |         st.thumbs.clear();
1193 |         st.thumb_gen = st.thumb_gen.wrapping_add(1);
1194 |     }
1195 |     rebuild_grid(ui);
1196 |     let gen = ui.state.borrow().thumb_gen;
1197 |     let shown = ui.state.borrow().shown.clone();
1198 |     load_thumbs(ui, shown, gen);
1199 | }
1200 | 
```
## start_search
```rust
1296 | fn start_search(ui: &Ui, query: String) {
1297 |     let q = query.to_lowercase();
1298 |     if q.is_empty() {
1299 |         let base = ui.state.borrow().base_shown.clone();
1300 |         if !base.is_empty() {
1301 |             let mut st = ui.state.borrow_mut();
1302 |             st.shown = base.clone();
1303 |             st.current = 0;
1304 |             st.thumbs.clear();
1305 |             st.pictures.clear();
1306 |             st.thumb_gen = st.thumb_gen.wrapping_add(1);
1307 |             drop(st);
1308 |             rebuild_grid(ui);
1309 |             let g = ui.state.borrow().thumb_gen;
1310 |             let sh = ui.state.borrow().shown.clone();
1311 |             load_thumbs(ui, sh, g);
1312 |         }
1313 |         return;
1314 |     }
1315 |     let gen = {
1316 |         let mut st = ui.state.borrow_mut();
1317 |         st.search_gen = st.search_gen.wrapping_add(1);
1318 |         st.search_gen
1319 |     };
1320 |     let index_ready = ui.index_ready.load(std::sync::atomic::Ordering::Acquire);
1321 |     let index_clone = if index_ready {
1322 |         Some(ui.search_index.lock().unwrap().clone())
1323 |     } else {
1324 |         None
1325 |     };
1326 |     let cfg = ui.config.borrow().clone();
1327 |     let (tx, rx) = std::sync::mpsc::channel();
1328 |     {
1329 |         let mut st = ui.state.borrow_mut();
1330 |         st.search_rx = Some(rx);
1331 |         st.search_pending = true;
1332 |         st.search_drainer_active = false;
1333 |     }
1334 |     std::thread::spawn(move || {
1335 |         if let Some(index) = index_clone {
1336 |             let shown: Vec<String> = index
1337 |                 .iter()
1338 |                 .filter(|(_, name)| name.contains(&q))
1339 |                 .take(20)
1340 |                 .map(|(p, _)| p.clone())
1341 |                 .collect();
1342 |             let _ = tx.send(SearchMsg::Result {
1343 |                 gen,
1344 |                 all: None,
1345 |                 index: None,
1346 |                 shown,
1347 |             });
1348 |         } else {
1349 |             let all = walk_images(&cfg.folder, cfg.subfolders, cfg.show_hidden);
1350 |             let index: Vec<(String, String)> =
1351 |                 all.iter().map(|p| (p.clone(), basename(p).to_lowercase())).collect();
1352 |             let shown: Vec<String> = index
1353 |                 .iter()
1354 |                 .filter(|(_, name)| name.contains(&q))
1355 |                 .take(20)
1356 |                 .map(|(p, _)| p.clone())
1357 |                 .collect();
1358 |             let _ = tx.send(SearchMsg::Result {
1359 |                 gen,
1360 |                 all: Some(all),
1361 |                 index: Some(index),
1362 |                 shown,
1363 |             });
1364 |         }
1365 |     });
1366 |     // drainer
1367 |     let ui2 = ui.clone();
1368 |     if !ui.state.borrow().search_drainer_active {
1369 |         ui.state.borrow_mut().search_drainer_active = true;
1370 |         glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
1371 |             let msg = {
1372 |                 let st = ui2.state.borrow();
1373 |                 st.search_rx.as_ref().and_then(|rx| rx.try_recv().ok())
1374 |             };
1375 |             if let Some(SearchMsg::Result {
1376 |                 gen,
1377 |                 all,
1378 |                 index,
1379 |                 shown,
1380 |             }) = msg
1381 |             {
1382 |                 let cur = ui2.state.borrow().search_gen;
1383 |                 if gen == cur {
1384 |                     {
1385 |                         let mut st = ui2.state.borrow_mut();
1386 |                         if let Some(a) = all {
1387 |                             st.all = a;
1388 |                             st.all_loaded = true;
1389 |                         }
1390 |                         if let Some(idx) = index {
1391 |                             *ui2.search_index.lock().unwrap() = idx;
1392 |                             ui2.index_ready.store(true, std::sync::atomic::Ordering::Release);
1393 |                             st.index_ready = true;
1394 |                         }
1395 |                         st.shown = shown.clone();
1396 |                         st.current = 0;
1397 |                         st.thumbs.clear();
1398 |                         st.pictures.clear();
1399 |                         st.thumb_gen = st.thumb_gen.wrapping_add(1);
1400 |                     }
1401 |                     rebuild_grid(&ui2);
1402 |                     let g = ui2.state.borrow().thumb_gen;
1403 |                     let sh = ui2.state.borrow().shown.clone();
1404 |                     load_thumbs(&ui2, sh, g);
1405 |                 }
1406 |                 ui2.state.borrow_mut().search_pending = false;
1407 |             }
1408 |             if ui2.state.borrow().search_pending {
1409 |                 glib::ControlFlow::Continue
1410 |             } else {
1411 |                 ui2.state.borrow_mut().search_rx = None;
1412 |                 ui2.state.borrow_mut().search_drainer_active = false;
1413 |                 glib::ControlFlow::Break
1414 |             }
1415 |         });
1416 |     }
1417 | }
```
## search_now
```rust
1419 | fn search_now(ui: &Ui) {
1420 |     let q = ui.entry.text().to_string();
1421 |     if q.trim().is_empty() {
1422 |         let base = ui.state.borrow().base_shown.clone();
1423 |         if !base.is_empty() {
1424 |             let mut st = ui.state.borrow_mut();
1425 |             st.shown = base;
1426 |             st.current = 0;
1427 |             st.thumbs.clear();
1428 |             st.pictures.clear();
1429 |             st.thumb_gen = st.thumb_gen.wrapping_add(1);
1430 |             drop(st);
1431 |             rebuild_grid(ui);
1432 |             let g = ui.state.borrow().thumb_gen;
1433 |             let sh = ui.state.borrow().shown.clone();
1434 |             load_thumbs(ui, sh, g);
1435 |         }
1436 |         return;
1437 |     }
1438 |     if !ui.state.borrow().index_ready {
1439 |         set_status(ui, "Indexing...");
1440 |     }
1441 |     start_search(ui, q);
1442 | }
```
## connect_signals (handlers)
```rust
1444 | fn connect_signals(ui: &Ui) {
1445 |     {
1446 |         let ui_c = ui.clone();
1447 |         ui.entry.connect_changed(move |_| {
1448 |             // Cancela el debounce anterior.
1449 |             if let Some(src) = ui_c.search_src.borrow_mut().take() {
1450 |                 src.remove();
1451 |             }
1452 |             let ui_t = ui_c.clone();
1453 |             let src = glib::timeout_add_local_once(Duration::from_millis(200), move || {
1454 |                 search_now(&ui_t);
1455 |             });
1456 |             *ui_c.search_src.borrow_mut() = Some(src);
1457 |         });
1458 |     }
1459 |     {
1460 |         let ui_s = ui.clone();
1461 |         ui.grid.connect_selected_children_changed(move |_| {
1462 |             if let Some(first) = ui_s.grid.selected_children().first() {
1463 |                 if let Ok(mut st) = ui_s.state.try_borrow_mut() {
1464 |                     st.current = first.index() as usize;
1465 |                 }
1466 |             }
1467 |         });
1468 |     }
1469 |     {
1470 |         let ui_a = ui.clone();
1471 |         ui.grid.connect_child_activated(move |_, _| apply_current(&ui_a));
1472 |     }
1473 |     // Cambios de opciones: persisten y (backend/fill/monitor/sort) reescanean.
1474 |     {
1475 |         let ui_o = ui.clone();
1476 |         ui.backend_drop.connect_selected_notify(move |d| {
1477 |             let avail = available_backends();
1478 |             let v = avail
1479 |                 .get(d.selected() as usize)
1480 |                 .map(|s| s.as_str())
1481 |                 .unwrap_or("swaybg");
1482 |             ui_o.config.borrow_mut().backend = v.to_string();
1483 |             save_config(&ui_o.config.borrow());
1484 |             reapply_wallpaper(&ui_o);
1485 |         });
1486 |     }
1487 |     {
1488 |         let ui_o = ui.clone();
1489 |         ui.monitor_drop.connect_selected_notify(move |d| {
1490 |             let mons = monitor_list();
1491 |             let v = mons.get(d.selected() as usize).map(|s| s.as_str()).unwrap_or("All");
1492 |             ui_o.config.borrow_mut().monitors = v.to_string();
1493 |             save_config(&ui_o.config.borrow());
1494 |             reapply_wallpaper(&ui_o);
1495 |         });
1496 |     }
1497 |     {
1498 |         let ui_o = ui.clone();
1499 |         let fills: Vec<&str> = FILLS.to_vec();
1500 |         ui.fill_drop.connect_selected_notify(move |d| {
1501 |             let v = fills.get(d.selected() as usize).unwrap_or(&"fill");
1502 |             ui_o.config.borrow_mut().fill = v.to_string();
1503 |             save_config(&ui_o.config.borrow());
1504 |             reapply_wallpaper(&ui_o);
1505 |         });
1506 |     }
1507 |     {
1508 |         let ui_o = ui.clone();
1509 |         let sort_modes = ["random", "modified", "created"];
1510 |         ui.sort_drop.connect_selected_notify(move |d| {
1511 |             let v = sort_modes.get(d.selected() as usize).copied().unwrap_or("random");
1512 |             ui_o.config.borrow_mut().sort_mode = v.to_string();
1513 |             save_config(&ui_o.config.borrow());
1514 |             resample(&ui_o);
1515 |         });
1516 |     }
1517 |     {
1518 |         let ui_o = ui.clone();
1519 |         ui.sub_check.connect_toggled(move |c| {
1520 |             ui_o.config.borrow_mut().subfolders = c.is_active();
1521 |             save_config(&ui_o.config.borrow());
1522 |             resample(&ui_o);
1523 |         });
1524 |     }
1525 |     {
1526 |         let ui_o = ui.clone();
1527 |         ui.hidden_check.connect_toggled(move |c| {
1528 |             ui_o.config.borrow_mut().show_hidden = c.is_active();
1529 |             save_config(&ui_o.config.borrow());
1530 |             resample(&ui_o);
1531 |         });
1532 |     }
1533 |     let key = gtk::EventControllerKey::new();
1534 |     key.set_propagation_phase(gtk::PropagationPhase::Capture);
1535 |     {
1536 |         let ui_k = ui.clone();
1537 |         key.connect_key_pressed(move |_, keyval, _, state| {
1538 |             let in_entry = ui_k.entry.has_focus();
1539 |             let ctrl_alt = state.contains(ModifierType::CONTROL_MASK)
1540 |                 || state.contains(ModifierType::ALT_MASK);
```
## build_ui Stack/toolbars/grids
```rust
1800 |     let grid = FlowBox::new();
1801 |     grid.add_css_class("bg-grid");
1802 |     grid.set_homogeneous(true);
1803 |     grid.set_selection_mode(gtk::SelectionMode::Single);
1804 |     grid.set_activate_on_single_click(true);
1805 |     scroll.set_child(Some(&grid));
1806 | 
1807 |     // Stack: página local + página Wallhaven (misma ventana, layer-shell).
1808 |     let view_stack = gtk::Stack::new();
1809 |     view_stack.set_vexpand(true);
1810 |     view_stack.set_hexpand(true);
1811 |     view_stack.add_named(&scroll, Some("local"));
1812 | 
1813 |     let local_toolbar = GtkBox::new(Orientation::Vertical, 0);
1814 |     local_toolbar.append(&bar);
1815 |     local_toolbar.append(&options);
1816 |     let toolbar_stack = gtk::Stack::new();
1817 |     toolbar_stack.set_hexpand(true);
1818 |     toolbar_stack.add_named(&local_toolbar, Some("local"));
1819 |     toolbar_stack.set_visible_child_name("local");
1820 | 
1821 |     view_stack.set_visible_child_name("local");
1822 | 
1823 |     let opt_popover = gtk::Popover::new();
1824 |     let opt_box = GtkBox::new(Orientation::Vertical, 6);
1825 |     let sort_label = Label::new(Some("Sort by"));
1826 |     sort_label.add_css_class("bg-optlabel");
1827 |     opt_box.append(&sort_label);
1828 |     opt_box.append(&sort_drop);
1829 |     opt_box.append(&sub_check);
1830 |     opt_box.append(&hidden_check);
1831 |     #[cfg(feature = "wallhaven")]
1832 |     {
1833 |         let btn_wh = Button::with_label("Wallhaven");
1834 |         btn_wh.add_css_class("bg-btn");
1835 |         opt_box.append(&btn_wh);
1836 |     }
1837 |     opt_popover.set_child(Some(&opt_box));
1838 |     btn_options.set_popover(Some(&opt_popover));
1839 | 
1840 |     card.append(&toolbar_stack);
1841 |     card.append(&view_stack);
1842 |     window.set_child(Some(&card));
1843 | 
1844 |     let provider = CssProvider::new();
1845 |     provider.load_from_string(&build_css());
1846 |     let display = gtk::gdk::Display::default().expect("aspire-bg: sin display");
1847 |     gtk::style_context_add_provider_for_display(
1848 |         &display,
1849 |         &provider,
1850 |         gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
1851 |     );
1852 | 
1853 |     let ui = Ui {
1854 |         app: app.clone(),
1855 |         window: window.clone(),
1856 |         bar: bar.clone(),
1857 |         options: options.clone(),
1858 |         entry: entry.clone(),
1859 |         status: status.clone(),
1860 |         provider: provider.clone(),
1861 |         grid: grid.clone(),
1862 |         scroll: scroll.clone(),
1863 |         backend_drop: backend_drop.clone(),
1864 |         monitor_drop: monitor_drop.clone(),
1865 |         fill_drop: fill_drop.clone(),
1866 |         sort_drop: sort_drop.clone(),
1867 |         sub_check: sub_check.clone(),
1868 |         hidden_check: hidden_check.clone(),
1869 |         state: Rc::new(RefCell::new(State {
1870 |             all: Vec::new(),
1871 |             all_loaded: false,
1872 |             shown: Vec::new(),
1873 |             base_shown: Vec::new(),
1874 |             current: 0,
1875 |             thumbs: HashMap::new(),
1876 |             pictures: HashMap::new(),
1877 |             thumb_gen: 0,
1878 |             search_gen: 0,
1879 |             search_pending: false,
1880 |             search_rx: None,
1881 |             search_drainer_active: false,
1882 |             sample_gen: 1,
1883 |             sample_pending: true,
1884 |             sample_rx: None,
1885 |             sample_drainer_active: false,
1886 |             index_ready: false,
1887 |         })),
1888 |         config: Rc::new(RefCell::new(config)),
1889 |         theme_system: theme_system.clone(),
1890 |         busy: Arc::new(AtomicBool::new(false)),
1891 |         search_src: Rc::new(RefCell::new(None)),
1892 |         search_index: Arc::new(Mutex::new(Vec::new())),
1893 |         index_ready: Arc::new(AtomicBool::new(false)),
1894 |     };
1895 | 
1896 |     #[cfg(feature = "wallhaven")]
1897 |     setup_wallhaven(&ui, &view_stack, &toolbar_stack, &btn_options);
1898 | 
1899 |     // Sincroniza los controles con la config (sin disparar rescans).
1900 |     // Si el motor guardado no está instalado, cae al primero disponible.
1901 |     {
1902 |         let avail = available_backends();
1903 |         let mut cfg = ui.config.borrow_mut();
1904 |         if !avail.iter().any(|b| *b == cfg.backend) {
1905 |             if let Some(first) = avail.first() {
1906 |                 cfg.backend = first.clone();
1907 |             }
1908 |         }
1909 |     }
1910 |     save_config(&ui.config.borrow());
1911 |     backend_drop.set_selected({
1912 |         let avail = available_backends();
1913 |         avail
1914 |             .iter()
1915 |             .position(|b| *b == ui.config.borrow().backend)
1916 |             .unwrap_or(0) as u32
1917 |     });
1918 |     monitor_drop.set_selected({
1919 |         let mons = monitor_list();
1920 |         mons.iter()
1921 |             .position(|m| *m == ui.config.borrow().monitors)
1922 |             .unwrap_or(0) as u32
1923 |     });
1924 |     fill_drop.set_selected(drop_index(&FILLS, &ui.config.borrow().fill));
1925 |     sort_drop.set_selected(match ui.config.borrow().sort_mode.as_str() {
1926 |         "modified" => 1,
1927 |         "created" => 2,
1928 |         _ => 0,
1929 |     });
1930 |     sub_check.set_active(ui.config.borrow().subfolders);
1931 |     hidden_check.set_active(ui.config.borrow().show_hidden);
1932 |     // NOTA: set_selected/set_active disparan los handlers de arriba; el
1933 |     // rescan resultante es inofensivo (misma carpeta y orden).
1934 |     if ui.config.borrow().zen_mode {
1935 |         bar.set_visible(false);
1936 |         options.set_visible(false);
1937 |     }
1938 | 
1939 |     {
1940 |         let ui_r = ui.clone();
1941 |         btn_random.connect_clicked(move |_| apply_random(&ui_r));
1942 |     }
1943 |     {
1944 |         let ui_r = ui.clone();
1945 |         btn_reshuffle.connect_clicked(move |_| reshuffle_20(&ui_r));
1946 |     }
1947 |     {
1948 |         let ui_f = ui.clone();
1949 |         btn_folder.connect_clicked(move |_| choose_folder(&ui_f));
1950 |     }
1951 |     connect_signals(&ui);
1952 |     rebuild_grid(&ui);
1953 |     launch_sample(&ui);
1954 |     if let Some(system) = &ui.theme_system {
1955 |         window.set_title(Some(&format!("Aspire Background — tema {system}")));
1956 |         set_status(
1957 |             &ui,
1958 |             &format!("Nuevo tema {system}: elige un fondo y pulsa Enter"),
1959 |         );
1960 |     }
1961 |     window.set_default_size(WIN_W, WIN_H);
1962 |     window.present();
1963 |     // Foco en el buscador como en aspire-menu/aspire-clip: se escribe y
1964 |     // filtra desde el primer momento; Esc lo suelta para navegar con hjkl.
1965 |     entry.grab_focus();
1966 | }
1967 | 
1968 | #[cfg(feature = "wallhaven")]
1969 | fn setup_wallhaven(ui: &Ui, view_stack: &gtk::Stack, toolbar_stack: &gtk::Stack, btn_options: &gtk::MenuButton) {
1970 |     use crate::wallhaven;
1971 |     // widgets
1972 |     let wh_scroll = ScrolledWindow::new();
1973 |     wh_scroll.set_policy(PolicyType::Never, PolicyType::Automatic);
1974 |     wh_scroll.set_has_frame(false);
1975 |     wh_scroll.set_vexpand(true);
1976 |     wh_scroll.set_hexpand(true);
1977 |     let wh_grid = FlowBox::new();
1978 |     wh_grid.add_css_class("bg-grid");
1979 |     wh_grid.set_homogeneous(true);
1980 |     wh_grid.set_selection_mode(gtk::SelectionMode::Single);
1981 |     wh_grid.set_activate_on_single_click(true);
1982 |     wh_scroll.set_child(Some(&wh_grid));
1983 | 
1984 |     let wh_top = GtkBox::new(Orientation::Horizontal, 8);
1985 |     let btn_back = Button::with_label("Back");
1986 |     btn_back.add_css_class("bg-btn");
1987 |     let wh_entry = Entry::new();
1988 |     wh_entry.set_placeholder_text(Some("Buscar en Wallhaven..."));
1989 |     wh_entry.add_css_class("bg-entry");
1990 |     wh_entry.set_hexpand(true);
1991 |     let cb_gen = CheckButton::with_label("General");
1992 |     let cb_anime = CheckButton::with_label("Anime");
1993 |     let cb_people = CheckButton::with_label("People");
1994 |     let wh_sort = DropDown::from_strings(&["Relevance", "Latest", "Random", "Views", "Favorites", "Toplist"]);
1995 |     let wh_purity = DropDown::from_strings(&["SFW", "Sketchy", "NSFW"]);
1996 |     let btn_more = Button::with_label("More");
1997 |     btn_more.add_css_class("bg-btn");
1998 |     wh_top.append(&btn_back);
1999 |     wh_top.append(&wh_entry);
2000 |     wh_top.append(&cb_gen);
2001 |     wh_top.append(&cb_anime);
2002 |     wh_top.append(&cb_people);
2003 |     wh_top.append(&wh_sort);
2004 |     wh_top.append(&wh_purity);
2005 |     wh_top.append(&btn_more);
2006 | 
2007 |     let wh_status = Label::new(None);
2008 |     wh_status.add_css_class("bg-status");
2009 |     wh_status.set_visible(false);
2010 |     wh_status.set_xalign(0.0);
```
## setup_wallhaven drainer + results
```rust
2095 |             drop(st);
2096 |             let ui = ui.clone();
2097 |             let wh_state = wh_state.clone();
2098 |             let wh_grid = wh_grid.clone();
2099 |             let wh_status = wh_status.clone();
2100 |             let btn_more = btn_more.clone();
2101 |             glib::timeout_add_local(Duration::from_millis(50), move || {
2102 |                 let has_rx = wh_state.borrow().rx.is_some();
2103 |                 if !has_rx {
2104 |                     wh_state.borrow_mut().drainer_active = false;
2105 |                     return glib::ControlFlow::Break;
2106 |                 }
2107 |                 loop {
2108 |                     let recv = {
2109 |                         let st = wh_state.borrow();
2110 |                         st.rx.as_ref().map(|r| r.try_recv())
2111 |                     };
2112 |                     match recv {
2113 |                         Some(Ok(ev)) => {
2114 |                             // handle ev below by binding
2115 |                             let ev = ev;
2116 |                             match ev {
2117 |                             WhEvent::Results {
2118 |                                 gen,
2119 |                                 page,
2120 |                                 last_page,
2121 |                                 items,
2122 |                             } => {
2123 |                                 let mut st = wh_state.borrow_mut();
2124 |                                 if st.gen != gen {
2125 |                                     continue;
2126 |                                 }
2127 |                                 if page == 1 {
2128 |                                     st.items.clear();
2129 |                                     st.seen.clear();
2130 |                                     st.thumbs.clear();
2131 |                                     st.pictures.clear();
2132 |                                 }
2133 |                                 for it in items {
2134 |                                     if st.seen.insert(it.id.clone()) {
2135 |                                         st.items.push(it);
2136 |                                     }
2137 |                                 }
2138 |                                 st.page = page;
2139 |                                 st.last_page = last_page;
2140 |                                 // rebuild grid
2141 |                                 while let Some(c) = wh_grid.first_child() {
2142 |                                     wh_grid.remove(&c);
2143 |                                 }
2144 |                                 st.pictures.clear();
2145 |                                 let items_clone: Vec<_> = st.items.clone();
2146 |                                 for it in &items_clone {
2147 |                                     let tex = st.thumbs.get(&it.id);
2148 |                                     let (tile, pic) = make_tile(&it.id, tex, false);
2149 |                                     wh_grid.append(&tile);
2150 |                                     st.pictures.insert(it.id.clone(), pic);
2151 |                                 }
2152 |                                 let can_more = st.page < st.max_pages && st.page < st.last_page;
2153 |                                 btn_more.set_sensitive(can_more);
2154 |                                 if st.items.is_empty() {
2155 |                                     wh_status.set_text("Sin resultados");
2156 |                                 } else {
2157 |                                     wh_status.set_text(&format!("{} resultados", st.items.len()));
2158 |                                 }
2159 |                                 wh_status.set_visible(true);
2160 |                             }
2161 |                             WhEvent::Thumb { gen, id, width, height, stride, bytes } => {
2162 |                                 let mut st = wh_state.borrow_mut();
2163 |                                 if st.gen != gen {
2164 |                                     continue;
2165 |                                 }
2166 |                                 let gbytes = glib::Bytes::from_owned(bytes);
2167 |                                 let tex = gtk::gdk::MemoryTexture::new(
2168 |                                     width,
2169 |                                     height,
2170 |                                     gtk::gdk::MemoryFormat::R8g8b8a8,
2171 |                                     &gbytes,
2172 |                                     stride,
2173 |                                 );
2174 |                                 st.thumbs.insert(id.clone(), tex.clone().into());
2175 |                                 if let Some(pic) = st.pictures.get(&id) {
2176 |                                     pic.set_paintable(Some(&tex));
2177 |                                 }
2178 |                             }
2179 |                             WhEvent::Error { gen, msg } => {
2180 |                                 let st = wh_state.borrow();
2181 |                                 if st.gen != gen {
2182 |                                     continue;
2183 |                                 }
2184 |                                 wh_status.set_text(&msg);
2185 |                                 wh_status.set_visible(true);
2186 |                             }
2187 |                             WhEvent::DownloadDone { path } => {
2188 |                                 let cfg = ui.config.borrow().clone();
2189 |                                 if let Err(e) = apply_wallpaper(&cfg, &path, false) {
2190 |                                     wh_status.set_text(&format!("Error: {e}"));
2191 |                                     wh_status.set_visible(true);
2192 |                                 } else {
2193 |                                     let dir = std::path::Path::new(&path).parent().unwrap();
2194 |                                     wallhaven::trim_wallhaven_dir(dir, &cfg.wallpaper, &path);
2195 |                                     {
2196 |                                         let mut c = ui.config.borrow_mut();
2197 |                                         c.wallpaper = path.clone();
2198 |                                         save_config(&c);
2199 |                                     }
2200 |                                     start_color_job(&ui, path.clone());
2201 |                                     wh_status.set_text("Fondo aplicado");
2202 |                                     wh_status.set_visible(true);
2203 |                                 }
2204 |                             }
2205 |                             WhEvent::DownloadError { msg } => {
2206 |                                 wh_status.set_text(&msg);
2207 |                                 wh_status.set_visible(true);
2208 |                             }
2209 |                         }
2210 |                     }
2211 |                     _ => break,
2212 |                 }
2213 |                 }
2214 |                 let pending = wh_state.borrow().pending.load(std::sync::atomic::Ordering::Acquire);
2215 |                 if pending == 0 {
2216 |                     let empty = wh_state
2217 |                         .borrow()
2218 |                         .rx
2219 |                         .as_ref()
2220 |                         .map(|r| r.try_recv().is_err())
2221 |                         .unwrap_or(true);
2222 |                     if empty {
2223 |                         let mut st = wh_state.borrow_mut();
2224 |                         st.rx = None;
2225 |                         st.tx = None;
2226 |                         st.drainer_active = false;
2227 |                         return glib::ControlFlow::Break;
2228 |                     }
2229 |                 }
2230 |                 glib::ControlFlow::Continue
2231 |             });
2232 |         })
2233 |     };
2234 | 
2235 |     // trigger download
```
## More handler
```rust
2478 |     // More
2479 |     {
2480 |         let start_search = start_search.clone();
2481 |         let btn_more = btn_more.clone();
2482 |         let wh_state = wh_state.clone();
2483 |         btn_more.connect_clicked(move |_| {
2484 |             let st = wh_state.borrow();
2485 |             if st.page >= st.max_pages || st.page >= st.last_page {
2486 |                 return;
2487 |             }
2488 |             drop(st);
2489 |             start_search(true);
2490 |         });
2491 |     }
2492 | 
2493 |     // Pick en grid
2494 |     {
2495 |         let wh_grid = wh_grid.clone();
2496 |         let wh_state = wh_state.clone();
```
## tile click handler
```rust
2448 |     {
2449 |         let start_search = start_search.clone();
2450 |         cb_gen.connect_toggled(move |_| start_search(false));
2451 |     }
2452 | 
2453 |     // Sort / Purity
2454 |     {
2455 |         let wh_state = wh_state.clone();
2456 |         let start_search = start_search.clone();
2457 |         let sort_map = ["Relevance", "Latest", "Random", "Views", "Favorites", "Toplist"];
2458 |         wh_sort.connect_selected_notify(move |d| {
2459 |             let v = sort_map.get(d.selected() as usize).copied().unwrap_or("Relevance");
2460 |             wh_state.borrow_mut().sort = v.to_string();
2461 |             start_search(false);
2462 |         });
2463 |     }
2464 |     {
2465 |         let wh_state = wh_state.clone();
2466 |         let start_search = start_search.clone();
2467 |         wh_purity.connect_selected_notify(move |d| {
2468 |             let purity = match d.selected() {
2469 |                 1 => "110",
2470 |                 2 => "111",
2471 |                 _ => "100",
2472 |             };
2473 |             wh_state.borrow_mut().purity = purity.to_string();
2474 |             start_search(false);
2475 |         });
2476 |     }
```
## wallhaven.rs search/download_full/download_thumb/trim
```rust
  25 | pub fn search(cfg: &crate::BgConfig, query: &str, categories: &str, sorting: &str, page: u32) -> Result<SearchPage, String> {
  26 |     let agent = agent();
  27 |     let mut req = agent
  28 |         .get("https://wallhaven.cc/api/v1/search")
  29 |         .query("categories", categories)
  30 |         .query("purity", &cfg.wallhaven.purity)
  31 |         .query("ratios", "16x9")
  32 |         .query("page", page.to_string());
  33 |     if !cfg.wallhaven.atleast.trim().is_empty() {
  34 |         req = req.query("atleast", cfg.wallhaven.atleast.trim());
  35 |     }
  36 |     if !query.trim().is_empty() {
  37 |         req = req.query("q", query.trim());
  38 |     }
  39 |     req = req.query("sorting", sorting);
  40 |     if !cfg.wallhaven.api_key.trim().is_empty() {
  41 |         req = req.header("X-API-Key", cfg.wallhaven.api_key.trim());
  42 |     }
  43 |     eprintln!("WH_SEARCH start q={:?} categories={} page={}", query, categories, page);
  44 |     let mut resp = req.call().map_err(|e| {
  45 |         if let ureq::Error::StatusCode(code) = e {
  46 |             match code {
  47 |                 401 => "Wallhaven: API key inválida o requerida (401)".to_string(),
  48 |                 429 => "Wallhaven: límite de peticiones (429)".to_string(),
  49 |                 _ => format!("Wallhaven: HTTP {code}"),
  50 |             }
  51 |         } else {
  52 |             "Wallhaven: sin red o error de conexión".to_string()
  53 |         }
  54 |     })?;
  55 |     if !resp.status().is_success() {
  56 |         return Err(format!("wallhaven: HTTP {}", resp.status()));
  57 |     }
  58 |     let text = resp.body_mut().read_to_string().map_err(|e| format!("wallhaven: {e}"))?;
  59 |     let json: serde_json::Value =
  60 |         serde_json::from_str(&text).map_err(|e| format!("Wallhaven: JSON inválido ({e})"))?;
  61 |     let data = json.get("data").and_then(|v| v.as_array()).cloned().unwrap_or_default();
  62 |     let mut items = Vec::new();
  63 |     for entry in data {
  64 |         let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
  65 |         if id.is_empty() {
  66 |             continue;
  67 |         }
  68 |         let url = entry.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string();
  69 |         let full_url = entry
  70 |             .get("path")
  71 |             .and_then(|v| v.as_str())
  72 |             .filter(|s| !s.is_empty())
  73 |             .unwrap_or(&url)
  74 |             .to_string();
  75 |         let thumb_url = entry
  76 |             .get("thumbs")
  77 |             .and_then(|v| v.get("small").or_else(|| v.get("large")))
  78 |             .and_then(|v| v.as_str())
  79 |             .unwrap_or("")
  80 |             .to_string();
  81 |         let ext = extension_of(&full_url);
  82 |         items.push(WhItem {
  83 |             id,
  84 |             full_url,
  85 |             thumb_url,
  86 |             ext,
  87 |         });
  88 |     }
  89 |     let last_page = json
  90 |         .get("meta")
  91 |         .and_then(|m| m.get("last_page"))
  92 |         .and_then(|v| v.as_u64())
  93 |         .unwrap_or(page as u64) as u32;
  94 |     Ok(SearchPage { items, last_page })
  95 | }
  96 | 
  97 | fn extension_of(url: &str) -> String {
  98 |     let path = url.split('?').next().unwrap_or(url);
  99 |     let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("jpg");
 100 |     let ext = ext.to_lowercase();
 101 |     if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp") {
 102 |         ext
 103 |     } else {
 104 |         "jpg".to_string()
 105 |     }
 106 | }
 107 | 
 108 | pub fn cache_dir() -> PathBuf {
 109 |     let home = std::env::var("HOME").unwrap_or_default();
 110 |     PathBuf::from(home).join(".cache/aspire-bg")
 111 | }
 112 | 
 113 | pub fn thumb_cache_path(url: &str) -> PathBuf {
 114 |     use std::hash::{DefaultHasher, Hash, Hasher};
 115 |     let mut h = DefaultHasher::new();
 116 |     url.hash(&mut h);
 117 |     cache_dir().join(format!("wh-{:016x}.{}", h.finish(), extension_of(url)))
 118 | }
 119 | 
 120 | pub fn download_thumb(url: &str) -> Result<PathBuf, String> {
 121 |     if url.is_empty() {
 122 |         return Err("thumb url vacía".to_string());
 123 |     }
 124 |     let path = thumb_cache_path(url);
 125 |     if path.is_file() {
 126 |         trim_cache();
 127 |         return Ok(path);
 128 |     }
 129 |     let agent = agent();
 130 |     let mut resp = agent.get(url).call().map_err(|e| format!("wallhaven thumb: {e}"))?;
 131 |     if !resp.status().is_success() {
 132 |         return Err(format!("wallhaven thumb: HTTP {}", resp.status()));
 133 |     }
 134 |     let bytes = resp.body_mut().read_to_vec().map_err(|e| format!("wallhaven thumb: {e}"))?;
 135 |     if let Some(parent) = path.parent() {
 136 |         let _ = std::fs::create_dir_all(parent);
 137 |     }
 138 |     std::fs::write(&path, &bytes).map_err(|e| format!("wallhaven thumb: {e}"))?;
 139 |     trim_cache();
 140 |     Ok(path)
 141 | }
 142 | 
 143 | pub fn download_full(item: &WhItem, dest_dir: &Path) -> Result<PathBuf, String> {
 144 |     std::fs::create_dir_all(dest_dir).map_err(|e| format!("wallhaven: {e}"))?;
 145 |     let safe_id: String = item.id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').collect();
 146 |     let ext = if item.ext.is_empty() { "jpg".to_string() } else { item.ext.clone() };
 147 |     let final_path = dest_dir.join(format!("wallhaven-{}.{}", safe_id, ext));
 148 |     if final_path.is_file() {
 149 |         return Ok(final_path);
 150 |     }
 151 |     let agent = agent();
 152 |     let mut resp = agent.get(&item.full_url).call().map_err(|e| format!("wallhaven: {e}"))?;
 153 |     if !resp.status().is_success() {
 154 |         return Err(format!("wallhaven: HTTP {}", resp.status()));
 155 |     }
 156 |     let tmp = dest_dir.join(format!(".wallhaven-{}.{}.part", safe_id, ext));
 157 |     let bytes = resp.body_mut().read_to_vec().map_err(|e| format!("wallhaven: {e}"))?;
 158 |     std::fs::write(&tmp, &bytes).map_err(|e| format!("wallhaven: {e}"))?;
 159 |     std::fs::rename(&tmp, &final_path).map_err(|e| format!("wallhaven: {e}"))?;
 160 |     Ok(final_path)
 161 | }
 162 | 
 163 | pub fn trim_cache() {
 164 |     let dir = cache_dir();
 165 |     let Ok(rd) = std::fs::read_dir(&dir) else { return; };
 166 |     let mut files: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();
 167 |     let mut total: u64 = 0;
 168 |     for e in rd.flatten() {
 169 |         let p = e.path();
 170 |         let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
 171 |         if !name.starts_with("wh-") {
 172 |             continue;
 173 |         }
 174 |         let Ok(meta) = p.metadata() else { continue; };
 175 |         if !meta.is_file() {
 176 |             continue;
 177 |         }
 178 |         let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
 179 |         let size = meta.len();
 180 |         total += size;
 181 |         files.push((p, mtime, size));
 182 |     }
 183 |     // límites: 200 archivos o 50 MB
 184 |     const MAX_FILES: usize = 200;
 185 |     const MAX_BYTES: u64 = 50 * 1024 * 1024;
 186 |     files.sort_by_key(|x| x.1);
 187 |     let mut i = 0;
 188 |     while (files.len().saturating_sub(i) > MAX_FILES || total > MAX_BYTES) && i < files.len() {
 189 |         let (p, _, size) = &files[i];
 190 |         let _ = std::fs::remove_file(p);
 191 |         total = total.saturating_sub(*size);
 192 |         i += 1;
 193 |     }
 194 | }
 195 | 
 196 | pub fn trim_wallhaven_dir(dir: &Path, applied: &str, new_file: &str) {
 197 |     let Ok(rd) = std::fs::read_dir(dir) else { return; };
 198 |     let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
 199 |     for e in rd.flatten() {
 200 |         let p = e.path();
 201 |         let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
 202 |         if !name.starts_with("wallhaven-") {
 203 |             continue;
 204 |         }
 205 |         let Ok(meta) = p.metadata() else { continue; };
 206 |         if !meta.is_file() {
 207 |             continue;
 208 |         }
 209 |         let path_str = p.to_string_lossy().to_string();
 210 |         if path_str == applied || path_str == new_file {
 211 |             continue;
 212 |         }
 213 |         let mtime = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
 214 |         files.push((p, mtime));
 215 |     }
 216 |     files.sort_by_key(|x| x.1);
 217 |     while files.len() + 2 > 30 && !files.is_empty() {
 218 |         // +2 = aplicado + nuevo (protegidos)
 219 |         if let Some((p, _)) = files.first() {
 220 |             let _ = std::fs::remove_file(p);
 221 |         }
 222 |         files.remove(0);
 223 |     }
```
## Cargo.toml
```toml
[package]
name = "aspire-bg"
version = "0.1.0"
edition = "2021"
description = "MangoWM wallpaper picker - GTK4 layer-shell grid, swaybg backend, system theme colors"

[features]
wallhaven = ["dep:ureq", "dep:serde_json"]

[dependencies]
aspire-core = { path = "../core" }
gtk = { package = "gtk4", version = "0.11", features = ["v4_12"] }
gtk4-layer-shell = "0.8"
gdk-pixbuf = "0.22"
dirs = "5.0"
ureq = { version = "3.4", default-features = false, features = ["rustls"], optional = true }
serde_json = { version = "1.0", optional = true }

[profile.release]
opt-level = 3
lto = true
strip = true
codegen-units = 1
panic = "abort"
```

## Appended verbatim (extra)
### make_tile
```rust
 948 | fn make_tile(_path: &str, tex: Option<&gtk::gdk::Texture>, is_current: bool) -> (GtkBox, Picture) {
 949 |     let tile = GtkBox::new(Orientation::Vertical, 0);
 950 |     tile.add_css_class("bg-tile");
 951 |     if is_current {
 952 |         tile.add_css_class("bg-current");
 953 |     }
 954 |     let picture = Picture::new();
 955 |     picture.set_content_fit(ContentFit::ScaleDown);
 956 |     picture.set_size_request(TILE_W, TILE_H);
 957 |     picture.add_css_class("bg-thumb");
 958 |     if let Some(t) = tex {
 959 |         picture.set_paintable(Some(t));
 960 |     }
 961 |     tile.append(&picture);
 962 |     (tile, picture)
 963 | }
```
### apply_current
```rust
1036 | fn apply_current(ui: &Ui) {
1037 |     let path = {
1038 |         let st = ui.state.borrow();
1039 |         match st.shown.get(st.current).cloned() {
1040 |             Some(p) => p,
1041 |             None => return,
1042 |         }
1043 |     };
1044 |     if let Err(e) = apply_wallpaper(&ui.config.borrow(), &path, false) {
1045 |         set_status(ui, &format!("Error al aplicar: {e}"));
1046 |         return;
1047 |     }
1048 |     rebuild_grid(ui);
1049 |     start_color_job(ui, path);
1050 | }
```
### apply_wallpaper
```rust
 596 | fn apply_wallpaper(cfg: &BgConfig, path: &str, no_post: bool) -> Result<(), String> {
 597 |     set_wallpaper(cfg, path)?;
 598 |     let mut cfg = cfg.clone();
 599 |     cfg.wallpaper = path.to_string();
 600 |     save_config(&cfg);
 601 |     if !no_post && !cfg.post_command.is_empty() {
 602 |         let cmd = cfg
 603 |             .post_command
 604 |             .replace("$wallpaper", path)
 605 |             .replace("$monitor", &cfg.monitors);
 606 |         let _ = std::process::Command::new("bash")
 607 |             .args(["-c", &cmd])
 608 |             .stdin(std::process::Stdio::null())
 609 |             .stdout(std::process::Stdio::null())
 610 |             .stderr(std::process::Stdio::null())
 611 |             .spawn();
 612 |     }
 613 |     Ok(())
 614 | }
```
### start_color_job
```rust
 887 | fn start_color_job(ui: &Ui, path: String) {
 888 |     if let Some(system) = ui.theme_system.clone() {
 889 |         run_color_job(
 890 |             ui,
 891 |             move || {
 892 |                 let scheme = theme_engine::create_tinty_scheme(Path::new(&path), &system)
 893 |                     .map_err(|e| format!("crear tema tinty: {e:#}"))?;
 894 |                 theme_engine::apply_tinty_scheme(&scheme)
 895 |                     .map_err(|e| format!("aplicar tema tinty: {e:#}"))?;
 896 |                 theme_engine::set_active_engine(Engine::Tinty)
 897 |                     .map_err(|e| format!("activar tinty: {e:#}"))?;
 898 |                 Ok(format!("Tema creado y aplicado: {scheme}"))
 899 |             },
 900 |             true,
 901 |         );
 902 |         return;
 903 |     }
 904 |     match theme_engine::active_engine() {
 905 |         Engine::Matugen => {
 906 |             run_color_job(
 907 |                 ui,
 908 |                 move || {
 909 |                     theme_engine::apply_matugen(Path::new(&path))
 910 |                         .map(|_| "Colores actualizados".to_string())
 911 |                         .map_err(|e| format!("{e:#}"))
 912 |                 },
 913 |                 false,
 914 |             );
 915 |         }
 916 |         Engine::Tinty => set_status(ui, "Fondo aplicado · tinty no cambia colores"),
 917 |     }
 918 | }
```
### start_search closure
```rust
1296 | fn start_search(ui: &Ui, query: String) {
1297 |     let q = query.to_lowercase();
1298 |     if q.is_empty() {
1299 |         let base = ui.state.borrow().base_shown.clone();
1300 |         if !base.is_empty() {
1301 |             let mut st = ui.state.borrow_mut();
1302 |             st.shown = base.clone();
1303 |             st.current = 0;
1304 |             st.thumbs.clear();
1305 |             st.pictures.clear();
1306 |             st.thumb_gen = st.thumb_gen.wrapping_add(1);
1307 |             drop(st);
1308 |             rebuild_grid(ui);
1309 |             let g = ui.state.borrow().thumb_gen;
1310 |             let sh = ui.state.borrow().shown.clone();
1311 |             load_thumbs(ui, sh, g);
1312 |         }
1313 |         return;
1314 |     }
1315 |     let gen = {
1316 |         let mut st = ui.state.borrow_mut();
1317 |         st.search_gen = st.search_gen.wrapping_add(1);
1318 |         st.search_gen
1319 |     };
1320 |     let index_ready = ui.index_ready.load(std::sync::atomic::Ordering::Acquire);
1321 |     let index_clone = if index_ready {
1322 |         Some(ui.search_index.lock().unwrap().clone())
1323 |     } else {
1324 |         None
1325 |     };
1326 |     let cfg = ui.config.borrow().clone();
1327 |     let (tx, rx) = std::sync::mpsc::channel();
1328 |     {
1329 |         let mut st = ui.state.borrow_mut();
1330 |         st.search_rx = Some(rx);
1331 |         st.search_pending = true;
1332 |         st.search_drainer_active = false;
1333 |     }
1334 |     std::thread::spawn(move || {
1335 |         if let Some(index) = index_clone {
1336 |             let shown: Vec<String> = index
1337 |                 .iter()
1338 |                 .filter(|(_, name)| name.contains(&q))
1339 |                 .take(20)
1340 |                 .map(|(p, _)| p.clone())
1341 |                 .collect();
1342 |             let _ = tx.send(SearchMsg::Result {
1343 |                 gen,
1344 |                 all: None,
1345 |                 index: None,
1346 |                 shown,
1347 |             });
1348 |         } else {
1349 |             let all = walk_images(&cfg.folder, cfg.subfolders, cfg.show_hidden);
1350 |             let index: Vec<(String, String)> =
1351 |                 all.iter().map(|p| (p.clone(), basename(p).to_lowercase())).collect();
1352 |             let shown: Vec<String> = index
1353 |                 .iter()
1354 |                 .filter(|(_, name)| name.contains(&q))
1355 |                 .take(20)
1356 |                 .map(|(p, _)| p.clone())
1357 |                 .collect();
1358 |             let _ = tx.send(SearchMsg::Result {
1359 |                 gen,
1360 |                 all: Some(all),
1361 |                 index: Some(index),
1362 |                 shown,
1363 |             });
1364 |         }
1365 |     });
1366 |     // drainer
1367 |     let ui2 = ui.clone();
1368 |     if !ui.state.borrow().search_drainer_active {
1369 |         ui.state.borrow_mut().search_drainer_active = true;
1370 |         glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
1371 |             let msg = {
1372 |                 let st = ui2.state.borrow();
1373 |                 st.search_rx.as_ref().and_then(|rx| rx.try_recv().ok())
1374 |             };
1375 |             if let Some(SearchMsg::Result {
1376 |                 gen,
1377 |                 all,
1378 |                 index,
1379 |                 shown,
1380 |             }) = msg
1381 |             {
1382 |                 let cur = ui2.state.borrow().search_gen;
1383 |                 if gen == cur {
1384 |                     {
1385 |                         let mut st = ui2.state.borrow_mut();
1386 |                         if let Some(a) = all {
1387 |                             st.all = a;
1388 |                             st.all_loaded = true;
1389 |                         }
1390 |                         if let Some(idx) = index {
1391 |                             *ui2.search_index.lock().unwrap() = idx;
1392 |                             ui2.index_ready.store(true, std::sync::atomic::Ordering::Release);
1393 |                             st.index_ready = true;
1394 |                         }
1395 |                         st.shown = shown.clone();
1396 |                         st.current = 0;
1397 |                         st.thumbs.clear();
1398 |                         st.pictures.clear();
1399 |                         st.thumb_gen = st.thumb_gen.wrapping_add(1);
1400 |                     }
1401 |                     rebuild_grid(&ui2);
1402 |                     let g = ui2.state.borrow().thumb_gen;
1403 |                     let sh = ui2.state.borrow().shown.clone();
1404 |                     load_thumbs(&ui2, sh, g);
1405 |                 }
1406 |                 ui2.state.borrow_mut().search_pending = false;
1407 |             }
1408 |             if ui2.state.borrow().search_pending {
1409 |                 glib::ControlFlow::Continue
1410 |             } else {
1411 |                 ui2.state.borrow_mut().search_rx = None;
1412 |                 ui2.state.borrow_mut().search_drainer_active = false;
1413 |                 glib::ControlFlow::Break
1414 |             }
1415 |         });
1416 |     }
1417 | }
```
### trigger_download block
```rust
2236 |     let trigger_download = {
2237 |         let wh_state = wh_state.clone();
2238 |         let start_drainer = start_drainer.clone();
2239 |         Rc::new(move |item: wallhaven::WhItem| {
2240 |             let tx = {
2241 |                 let mut st = wh_state.borrow_mut();
2242 |                 if st.tx.is_none() {
2243 |                     let (t, r) = std::sync::mpsc::channel();
2244 |                     st.tx = Some(t.clone());
2245 |                     st.rx = Some(r);
2246 |                 }
2247 |                 st.tx.clone().unwrap()
2248 |             };
2249 |             let pending = wh_state.borrow().pending.clone();
2250 |             pending.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
2251 |             let dest = std::path::PathBuf::from(format!(
2252 |                 "{}/.config/aspire-bg/wallhaven",
2253 |                 std::env::var("HOME").unwrap_or_default()
2254 |             ));
2255 |             let txw = tx.clone();
2256 |             let item_clone = item.clone();
2257 |             std::thread::spawn(move || {
2258 |                 match wallhaven::download_full(&item_clone, &dest) {
2259 |                     Ok(path) => {
2260 |                         txw.send(WhEvent::DownloadDone {
2261 |                             path: path.to_string_lossy().to_string(),
2262 |                         })
2263 |                         .ok();
2264 |                     }
2265 |                     Err(e) => {
2266 |                         txw.send(WhEvent::DownloadError { msg: e }).ok();
2267 |                     }
2268 |                 }
2269 |                 pending.fetch_sub(1, std::sync::atomic::Ordering::Release);
2270 |             });
```
### tile click handler
```rust
2497 |         wh_grid.connect_child_activated(move |_, child| {
2498 |             let idx = child.index() as usize;
2499 |             let item = {
2500 |                 let st = wh_state.borrow();
2501 |                 st.items.get(idx).cloned()
2502 |             };
2503 |             let Some(item) = item else { return; };
2504 |             trigger_download(item);
2505 |             let _ = wh_status;
2506 |             let _ = ui;
2507 |         });
2508 |     }
2509 | }
```
### connect_signals (local)
```rust
1444 | fn connect_signals(ui: &Ui) {
1445 |     {
1446 |         let ui_c = ui.clone();
1447 |         ui.entry.connect_changed(move |_| {
1448 |             // Cancela el debounce anterior.
1449 |             if let Some(src) = ui_c.search_src.borrow_mut().take() {
1450 |                 src.remove();
1451 |             }
1452 |             let ui_t = ui_c.clone();
1453 |             let src = glib::timeout_add_local_once(Duration::from_millis(200), move || {
1454 |                 search_now(&ui_t);
1455 |             });
1456 |             *ui_c.search_src.borrow_mut() = Some(src);
1457 |         });
1458 |     }
1459 |     {
1460 |         let ui_s = ui.clone();
1461 |         ui.grid.connect_selected_children_changed(move |_| {
1462 |             if let Some(first) = ui_s.grid.selected_children().first() {
1463 |                 if let Ok(mut st) = ui_s.state.try_borrow_mut() {
1464 |                     st.current = first.index() as usize;
1465 |                 }
1466 |             }
1467 |         });
1468 |     }
1469 |     {
1470 |         let ui_a = ui.clone();
1471 |         ui.grid.connect_child_activated(move |_, _| apply_current(&ui_a));
1472 |     }
1473 |     // Cambios de opciones: persisten y (backend/fill/monitor/sort) reescanean.
1474 |     {
1475 |         let ui_o = ui.clone();
1476 |         ui.backend_drop.connect_selected_notify(move |d| {
1477 |             let avail = available_backends();
1478 |             let v = avail
1479 |                 .get(d.selected() as usize)
1480 |                 .map(|s| s.as_str())
1481 |                 .unwrap_or("swaybg");
1482 |             ui_o.config.borrow_mut().backend = v.to_string();
1483 |             save_config(&ui_o.config.borrow());
1484 |             reapply_wallpaper(&ui_o);
1485 |         });
1486 |     }
1487 |     {
1488 |         let ui_o = ui.clone();
1489 |         ui.monitor_drop.connect_selected_notify(move |d| {
1490 |             let mons = monitor_list();
1491 |             let v = mons.get(d.selected() as usize).map(|s| s.as_str()).unwrap_or("All");
1492 |             ui_o.config.borrow_mut().monitors = v.to_string();
1493 |             save_config(&ui_o.config.borrow());
1494 |             reapply_wallpaper(&ui_o);
1495 |         });
1496 |     }
1497 |     {
1498 |         let ui_o = ui.clone();
1499 |         let fills: Vec<&str> = FILLS.to_vec();
1500 |         ui.fill_drop.connect_selected_notify(move |d| {
1501 |             let v = fills.get(d.selected() as usize).unwrap_or(&"fill");
1502 |             ui_o.config.borrow_mut().fill = v.to_string();
1503 |             save_config(&ui_o.config.borrow());
1504 |             reapply_wallpaper(&ui_o);
1505 |         });
1506 |     }
1507 |     {
1508 |         let ui_o = ui.clone();
1509 |         let sort_modes = ["random", "modified", "created"];
1510 |         ui.sort_drop.connect_selected_notify(move |d| {
1511 |             let v = sort_modes.get(d.selected() as usize).copied().unwrap_or("random");
1512 |             ui_o.config.borrow_mut().sort_mode = v.to_string();
1513 |             save_config(&ui_o.config.borrow());
1514 |             resample(&ui_o);
1515 |         });
1516 |     }
1517 |     {
1518 |         let ui_o = ui.clone();
1519 |         ui.sub_check.connect_toggled(move |c| {
1520 |             ui_o.config.borrow_mut().subfolders = c.is_active();
1521 |             save_config(&ui_o.config.borrow());
1522 |             resample(&ui_o);
1523 |         });
1524 |     }
1525 |     {
1526 |         let ui_o = ui.clone();
1527 |         ui.hidden_check.connect_toggled(move |c| {
1528 |             ui_o.config.borrow_mut().show_hidden = c.is_active();
1529 |             save_config(&ui_o.config.borrow());
1530 |             resample(&ui_o);
1531 |         });
1532 |     }
1533 |     let key = gtk::EventControllerKey::new();
1534 |     key.set_propagation_phase(gtk::PropagationPhase::Capture);
1535 |     {
1536 |         let ui_k = ui.clone();
1537 |         key.connect_key_pressed(move |_, keyval, _, state| {
1538 |             let in_entry = ui_k.entry.has_focus();
1539 |             let ctrl_alt = state.contains(ModifierType::CONTROL_MASK)
1540 |                 || state.contains(ModifierType::ALT_MASK);
1541 |             // Letras de waypaper (solo fuera del buscador).
1542 |             if !in_entry && !ctrl_alt {
1543 |                 match keyval {
1544 |                     Key::h => { move_selection(&ui_k, -1, false); return Propagation::Stop; }
1545 |                     Key::l => { move_selection(&ui_k, 1, false); return Propagation::Stop; }
1546 |                     Key::j => { move_selection(&ui_k, 1, true); return Propagation::Stop; }
1547 |                     Key::k => { move_selection(&ui_k, -1, true); return Propagation::Stop; }
1548 |                     Key::g => { scroll_edge(&ui_k, true); return Propagation::Stop; }
1549 |                     Key::G => { scroll_edge(&ui_k, false); return Propagation::Stop; }
1550 |                     Key::r => {
1551 |                         // Recarga caché de miniaturas y reescanea.
1552 |                         let _ = std::fs::remove_dir_all(format!("{}/.cache/aspire-bg", home()));
1553 |                         rescan(&ui_k);
1554 |                         return Propagation::Stop;
1555 |                     }
1556 |                     Key::R => {
1557 |                         // Mayúscula = fondo aleatorio.
1558 |                         apply_random(&ui_k);
1559 |                         return Propagation::Stop;
1560 |                     }
1561 |                     Key::f => { choose_folder(&ui_k); return Propagation::Stop; }
1562 |                     Key::s => {
1563 |                         let v = !ui_k.config.borrow().subfolders;
1564 |                         ui_k.sub_check.set_active(v);
1565 |                         return Propagation::Stop;
1566 |                     }
1567 |                     Key::period => {
1568 |                         let v = !ui_k.config.borrow().show_hidden;
1569 |                         ui_k.hidden_check.set_active(v);
1570 |                         return Propagation::Stop;
1571 |                     }
1572 |                     Key::slash => { ui_k.entry.grab_focus(); return Propagation::Stop; }
1573 |                     Key::z => {
1574 |                         let v = !ui_k.config.borrow().zen_mode;
1575 |                         set_zen(&ui_k, v);
1576 |                         return Propagation::Stop;
1577 |                     }
1578 |                     Key::question => { show_help(&ui_k.window); return Propagation::Stop; }
1579 |                     Key::q => { ui_k.app.quit(); return Propagation::Stop; }
1580 |                     _ => {}
1581 |                 }
1582 |             }
1583 |             if state.contains(ModifierType::CONTROL_MASK) && keyval == Key::r {
1584 |                 reshuffle_20(&ui_k);
1585 |                 return Propagation::Stop;
1586 |             }
1587 |             if keyval == Key::Escape {
1588 |                 if in_entry {
1589 |                     ui_k.entry.set_text("");
1590 |                     if let Some(child) = ui_k.grid.child_at_index(0) {
1591 |                         child.grab_focus();
1592 |                     }
1593 |                 } else {
1594 |                     ui_k.app.quit();
1595 |                 }
1596 |                 Propagation::Stop
1597 |             } else if keyval == Key::Return || keyval == Key::KP_Enter {
1598 |                 apply_current(&ui_k);
1599 |                 Propagation::Stop
1600 |             } else if keyval == Key::Up {
1601 |                 move_selection(&ui_k, -1, true);
1602 |                 Propagation::Stop
1603 |             } else if keyval == Key::Down {
1604 |                 move_selection(&ui_k, 1, true);
1605 |                 Propagation::Stop
1606 |             } else if keyval == Key::Left {
1607 |                 move_selection(&ui_k, -1, false);
1608 |                 Propagation::Stop
1609 |             } else if keyval == Key::Right {
1610 |                 move_selection(&ui_k, 1, false);
1611 |                 Propagation::Stop
1612 |             } else {
1613 |                 Propagation::Proceed
1614 |             }
1615 |         });
1616 |     }
1617 |     ui.window.add_controller(key);
1618 | }
```
### drainer closure
```rust
2101 |             glib::timeout_add_local(Duration::from_millis(50), move || {
2102 |                 let has_rx = wh_state.borrow().rx.is_some();
2103 |                 if !has_rx {
2104 |                     wh_state.borrow_mut().drainer_active = false;
2105 |                     return glib::ControlFlow::Break;
2106 |                 }
2107 |                 loop {
2108 |                     let recv = {
2109 |                         let st = wh_state.borrow();
2110 |                         st.rx.as_ref().map(|r| r.try_recv())
2111 |                     };
2112 |                     match recv {
2113 |                         Some(Ok(ev)) => {
2114 |                             // handle ev below by binding
2115 |                             let ev = ev;
2116 |                             match ev {
2117 |                             WhEvent::Results {
2118 |                                 gen,
2119 |                                 page,
2120 |                                 last_page,
2121 |                                 items,
2122 |                             } => {
2123 |                                 let mut st = wh_state.borrow_mut();
2124 |                                 if st.gen != gen {
2125 |                                     continue;
2126 |                                 }
2127 |                                 if page == 1 {
2128 |                                     st.items.clear();
2129 |                                     st.seen.clear();
2130 |                                     st.thumbs.clear();
2131 |                                     st.pictures.clear();
2132 |                                 }
2133 |                                 for it in items {
2134 |                                     if st.seen.insert(it.id.clone()) {
2135 |                                         st.items.push(it);
2136 |                                     }
2137 |                                 }
2138 |                                 st.page = page;
2139 |                                 st.last_page = last_page;
2140 |                                 // rebuild grid
2141 |                                 while let Some(c) = wh_grid.first_child() {
2142 |                                     wh_grid.remove(&c);
2143 |                                 }
2144 |                                 st.pictures.clear();
2145 |                                 let items_clone: Vec<_> = st.items.clone();
2146 |                                 for it in &items_clone {
2147 |                                     let tex = st.thumbs.get(&it.id);
2148 |                                     let (tile, pic) = make_tile(&it.id, tex, false);
2149 |                                     wh_grid.append(&tile);
2150 |                                     st.pictures.insert(it.id.clone(), pic);
2151 |                                 }
2152 |                                 let can_more = st.page < st.max_pages && st.page < st.last_page;
2153 |                                 btn_more.set_sensitive(can_more);
2154 |                                 if st.items.is_empty() {
2155 |                                     wh_status.set_text("Sin resultados");
2156 |                                 } else {
2157 |                                     wh_status.set_text(&format!("{} resultados", st.items.len()));
2158 |                                 }
2159 |                                 wh_status.set_visible(true);
2160 |                             }
2161 |                             WhEvent::Thumb { gen, id, width, height, stride, bytes } => {
2162 |                                 let mut st = wh_state.borrow_mut();
2163 |                                 if st.gen != gen {
2164 |                                     continue;
2165 |                                 }
2166 |                                 let gbytes = glib::Bytes::from_owned(bytes);
2167 |                                 let tex = gtk::gdk::MemoryTexture::new(
2168 |                                     width,
2169 |                                     height,
2170 |                                     gtk::gdk::MemoryFormat::R8g8b8a8,
2171 |                                     &gbytes,
2172 |                                     stride,
2173 |                                 );
2174 |                                 st.thumbs.insert(id.clone(), tex.clone().into());
2175 |                                 if let Some(pic) = st.pictures.get(&id) {
2176 |                                     pic.set_paintable(Some(&tex));
2177 |                                 }
2178 |                             }
2179 |                             WhEvent::Error { gen, msg } => {
2180 |                                 let st = wh_state.borrow();
2181 |                                 if st.gen != gen {
2182 |                                     continue;
2183 |                                 }
2184 |                                 wh_status.set_text(&msg);
2185 |                                 wh_status.set_visible(true);
2186 |                             }
2187 |                             WhEvent::DownloadDone { path } => {
2188 |                                 let cfg = ui.config.borrow().clone();
2189 |                                 if let Err(e) = apply_wallpaper(&cfg, &path, false) {
2190 |                                     wh_status.set_text(&format!("Error: {e}"));
2191 |                                     wh_status.set_visible(true);
2192 |                                 } else {
2193 |                                     let dir = std::path::Path::new(&path).parent().unwrap();
2194 |                                     wallhaven::trim_wallhaven_dir(dir, &cfg.wallpaper, &path);
2195 |                                     {
2196 |                                         let mut c = ui.config.borrow_mut();
2197 |                                         c.wallpaper = path.clone();
2198 |                                         save_config(&c);
2199 |                                     }
2200 |                                     start_color_job(&ui, path.clone());
2201 |                                     wh_status.set_text("Fondo aplicado");
2202 |                                     wh_status.set_visible(true);
2203 |                                 }
2204 |                             }
2205 |                             WhEvent::DownloadError { msg } => {
2206 |                                 wh_status.set_text(&msg);
2207 |                                 wh_status.set_visible(true);
2208 |                             }
2209 |                         }
2210 |                     }
2211 |                     _ => break,
2212 |                 }
2213 |                 }
2214 |                 let pending = wh_state.borrow().pending.load(std::sync::atomic::Ordering::Acquire);
2215 |                 if pending == 0 {
2216 |                     let empty = wh_state
2217 |                         .borrow()
2218 |                         .rx
2219 |                         .as_ref()
2220 |                         .map(|r| r.try_recv().is_err())
2221 |                         .unwrap_or(true);
2222 |                     if empty {
2223 |                         let mut st = wh_state.borrow_mut();
2224 |                         st.rx = None;
2225 |                         st.tx = None;
2226 |                         st.drainer_active = false;
2227 |                         return glib::ControlFlow::Break;
2228 |                     }
2229 |                 }
2230 |                 glib::ControlFlow::Continue
2231 |             });
2232 |         })
2233 |     };
2234 | 
2235 |     // trigger download
```
