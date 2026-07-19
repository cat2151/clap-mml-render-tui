use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
};

use super::*;

fn unique_temp_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "cmrt-voicing-source-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn json_entry(patch: &str, voicing: &str) -> String {
    format!(r#"{{"entries":{{"{patch}":"{voicing}"}}}}"#)
}

fn spawn_refresh_for_test(sources: SourceSet) -> VoicingSourceRefresh {
    let refresh = VoicingSourceRefresh {
        sources: Some(sources),
        completion: Arc::new((Mutex::new(false), Condvar::new())),
        io_lock: Arc::new(Mutex::new(())),
    };
    refresh.spawn_worker();
    refresh
}

#[test]
fn voicing_layers_resolve_override_then_user_then_shared() {
    let mut shared = VoicingCache::default();
    let mut user = VoicingCache::default();
    let mut override_ = VoicingCache::default();
    shared.insert("Leads/Shared.fxp", PatchVoicing::Poly);
    shared.insert("Leads/User.fxp", PatchVoicing::Poly);
    shared.insert("Leads/Override.fxp", PatchVoicing::Poly);
    user.insert("Leads/User.fxp", PatchVoicing::Mono);
    user.insert("Leads/Override.fxp", PatchVoicing::Poly);
    override_.insert("Leads/Override.fxp", PatchVoicing::Mono);
    let layers = VoicingLayers { shared, override_ };

    assert_eq!(
        layers.resolve(&user, "Leads/Override.fxp"),
        Some(PatchVoicing::Mono)
    );
    assert_eq!(
        layers.resolve(&user, "Leads/User.fxp"),
        Some(PatchVoicing::Mono)
    );
    assert_eq!(
        layers.resolve(&user, "Leads/Shared.fxp"),
        Some(PatchVoicing::Poly)
    );
}

#[test]
fn shared_json_normalizes_case_and_separators() {
    let cache = VoicingCache::from_shared_json(
        r#"{"entries":{"Patches_3rdParty\\Slowboat/Winds/Clarinet.fxp":"mono"}}"#,
    )
    .unwrap();

    assert_eq!(
        cache.get("patches_3rdparty/slowboat/winds/clarinet.fxp"),
        Some(PatchVoicing::Mono)
    );
}

#[test]
fn local_source_is_resolved_from_config_dir_and_only_rewritten_when_changed() {
    let temp = unique_temp_dir("local");
    fs::create_dir_all(temp.join("data")).unwrap();
    let source_path = temp.join("data/shared.json");
    fs::write(
        &source_path,
        json_entry("Leads/Local.fxp", "mono").as_bytes(),
    )
    .unwrap();
    let sources = SourceSet::new(&temp, "data/shared.json".to_string(), String::new());
    let io_lock = Mutex::new(());

    refresh_local_source(&sources.shared, &io_lock).unwrap();
    let first = fs::read(&sources.shared.data_path).unwrap();
    assert!(!write_if_changed(&sources.shared.data_path, &first).unwrap());

    fs::write(
        &source_path,
        json_entry("Leads/Local.fxp", "poly").as_bytes(),
    )
    .unwrap();
    refresh_local_source(&sources.shared, &io_lock).unwrap();
    assert_eq!(
        load_persisted_cache(&sources.shared).get("Leads/Local.fxp"),
        Some(PatchVoicing::Poly)
    );
    fs::remove_dir_all(temp).ok();
}

#[test]
fn invalid_local_json_keeps_previous_persistent_copy() {
    let temp = unique_temp_dir("invalid");
    fs::create_dir_all(&temp).unwrap();
    let source_path = temp.join("source.json");
    let valid = json_entry("Leads/Valid.fxp", "mono");
    fs::write(&source_path, valid.as_bytes()).unwrap();
    let sources = SourceSet::new(&temp, "source.json".to_string(), String::new());
    let io_lock = Mutex::new(());
    refresh_local_source(&sources.shared, &io_lock).unwrap();

    fs::write(&source_path, b"not json").unwrap();
    assert!(refresh_local_source(&sources.shared, &io_lock).is_err());
    assert_eq!(
        fs::read_to_string(&sources.shared.data_path).unwrap(),
        valid
    );
    fs::remove_dir_all(temp).ok();
}

#[test]
fn first_keyboard_entry_waits_for_missing_local_source() {
    let temp = unique_temp_dir("first-load");
    fs::create_dir_all(&temp).unwrap();
    fs::write(
        temp.join("shared-source.json"),
        json_entry("Leads/First.fxp", "mono"),
    )
    .unwrap();
    let sources = SourceSet::new(&temp, "shared-source.json".to_string(), String::new());
    let refresh = spawn_refresh_for_test(sources);

    let layers = refresh.load_for_keyboard();

    assert_eq!(
        layers.resolve(&VoicingCache::default(), "Leads/First.fxp"),
        Some(PatchVoicing::Mono)
    );
    fs::remove_dir_all(temp).ok();
}

#[test]
fn first_keyboard_entry_continues_when_source_is_unavailable() {
    let temp = unique_temp_dir("first-failure");
    let sources = SourceSet::new(&temp, "missing.json".to_string(), String::new());
    let refresh = spawn_refresh_for_test(sources);

    let layers = refresh.load_for_keyboard();

    assert_eq!(
        layers.resolve(&VoicingCache::default(), "Leads/Missing.fxp"),
        None
    );
    fs::remove_dir_all(temp).ok();
}

#[test]
fn url_source_uses_etag_and_preserves_json_on_304() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (request_tx, request_rx) = mpsc::channel();
    let body = json_entry("Leads/Remote.fxp", "poly");
    let server = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = vec![0_u8; 4096];
            let count = stream.read(&mut request).unwrap();
            request_tx
                .send(String::from_utf8_lossy(&request[..count]).to_string())
                .unwrap();
            if index == 0 {
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nETag: \"v1\"\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            } else {
                stream
                    .write_all(
                        b"HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nConnection: close\r\n\r\n",
                    )
                    .unwrap();
            }
        }
    });
    let temp = unique_temp_dir("etag");
    let sources = SourceSet::new(
        &temp,
        format!("http://{address}/voicing.json"),
        String::new(),
    );
    let io_lock = Mutex::new(());

    refresh_url_source(&sources.shared, &io_lock).unwrap();
    let persisted = fs::read(&sources.shared.data_path).unwrap();
    refresh_url_source(&sources.shared, &io_lock).unwrap();
    assert_eq!(fs::read(&sources.shared.data_path).unwrap(), persisted);
    server.join().unwrap();

    let first = request_rx.recv().unwrap();
    let second = request_rx.recv().unwrap();
    assert!(!first.to_ascii_lowercase().contains("if-none-match"));
    assert!(second.contains("If-None-Match: \"v1\""));
    fs::remove_dir_all(temp).ok();
}

#[test]
fn persisted_sources_are_read_again_for_each_keyboard_entry() {
    let temp = unique_temp_dir("reload");
    let sources = SourceSet::new(&temp, String::new(), "override-source".to_string());
    fs::create_dir_all(sources.override_.data_path.parent().unwrap()).unwrap();
    fs::write(
        &sources.override_.data_path,
        json_entry("Winds/Reload.fxp", "poly"),
    )
    .unwrap();
    let first = load_persisted_cache(&sources.override_);
    fs::write(
        &sources.override_.data_path,
        json_entry("Winds/Reload.fxp", "mono"),
    )
    .unwrap();
    let second = load_persisted_cache(&sources.override_);

    assert_eq!(first.get("Winds/Reload.fxp"), Some(PatchVoicing::Poly));
    assert_eq!(second.get("Winds/Reload.fxp"), Some(PatchVoicing::Mono));
    fs::remove_dir_all(temp).ok();
}
