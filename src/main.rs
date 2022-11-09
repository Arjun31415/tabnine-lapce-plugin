use anyhow::Result;
use lapce_plugin::{
    psp_types::{
        lsp_types::{
            request::Initialize, DocumentFilter, DocumentSelector, InitializeParams, MessageType,
            Url,
        },
        Request,
    },
    register_plugin, Http, LapcePlugin, VoltEnvironment, PLUGIN_RPC,
};
use serde_json::Value;
use std::fs::File;
use zip::ZipArchive;

#[derive(Default)]
struct State {}

register_plugin!(State);
macro_rules! string {
    ( $x:expr ) => {
        String::from($x)
    };
}
fn initialize(params: InitializeParams) -> Result<()> {
    let document_selector: DocumentSelector = vec![DocumentFilter {
        // lsp language id
        language: Some(String::from("")),
        // glob pattern
        pattern: Some(String::from("**/*")),
        // like file:
        scheme: None,
    }];
    let mut server_args = vec![];

    // Check for user specified LSP server path
    // ```
    // [lapce-plugin-name.lsp]
    // serverPath = "[path or filename]"
    // serverArgs = ["--arg1", "--arg2"]
    // ```
    if let Some(options) = params.initialization_options.as_ref() {
        if let Some(lsp) = options.get("lsp") {
            if let Some(args) = lsp.get("serverArgs") {
                if let Some(args) = args.as_array() {
                    if !args.is_empty() {
                        server_args = vec![];
                    }
                    for arg in args {
                        if let Some(arg) = arg.as_str() {
                            server_args.push(arg.to_string());
                        }
                    }
                }
            }

            if let Some(server_path) = lsp.get("serverPath") {
                if let Some(server_path) = server_path.as_str() {
                    if !server_path.is_empty() {
                        let server_uri = Url::parse(&format!("urn:{}", server_path))?;
                        PLUGIN_RPC.start_lsp(
                            server_uri,
                            server_args,
                            document_selector,
                            params.initialization_options,
                        );
                        return Ok(());
                    }
                }
            }
        }
    }
    let url = "https://update.tabnine.com/bundles/version";
    PLUGIN_RPC.stderr("Starting tabnine version check");
    let mut resp = Http::get(url)?;
    let version = if resp.status_code.is_success() {
        let body = resp.body_read_all()?;
        String::from_utf8(body).unwrap()
    } else {
        panic!("Response error: {}", resp.status_code);
    };
    PLUGIN_RPC.stderr(&format!("Tabine Latest Version: {}", version));

    // Architecture check
    let platform = match VoltEnvironment::operating_system().as_deref() {
        Ok("macos") => "apple-darwin",
        Ok("linux") => "unknown-linux-gnu",
        Ok("windows") => "pc-windows-gnu",
        p => panic!("unsupported platform {:#?}", p),
    };

    let arch = match VoltEnvironment::architecture().as_deref() {
        Ok("x86_64") => "x86_64",
        Ok("aarch64") => "aarch64",
        a => panic!("unsupported architecture {:#?}", a),
    };
    let download_url = format!(
        "https://update.tabnine.com/bundles/{}/{}-{}/TabNine.zip",
        version, arch, platform
    );
    PLUGIN_RPC.stderr(&format!("Downloading Tabnine from {}", download_url));
    let mut resp = Http::get(&download_url)?;
    let zip_file = format!("TabNine-{}-{}-{}.zip", version, arch, platform);
    if resp.status_code.is_success() {
        let body = resp.body_read_all()?;
        std::fs::write(&zip_file, body).unwrap();
        let mut zip = ZipArchive::new(File::open(&zip_file).unwrap()).unwrap();
        // for every zip file
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).unwrap();
            let outpath = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };
            PLUGIN_RPC.stderr(&format!("Extracting {}", outpath.display()));
            if (*file.name()).ends_with('/') {
                std::fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p).unwrap();
                    }
                }
                let mut outfile = std::fs::File::create(&outpath).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }
        }
        std::fs::remove_file(&zip_file).unwrap();
    } else {
        panic!("Response error: {}", resp.status_code);
    }
    PLUGIN_RPC.stderr("Finished extraction");
    let filename = match VoltEnvironment::operating_system().as_deref() {
        Ok("windows") => {
            string!("TabNine.exe")
        }
        _ => string!("TabNine"),
    };
    // see lapce_plugin::Http for available API to download files

    // Plugin working directory
    let volt_uri = VoltEnvironment::uri()?;
    let server_path = Url::parse(&volt_uri)?.join(&filename)?;

    // Available language IDs
    // https://github.com/lapce/lapce/blob/HEAD/lapce-proxy/src/buffer.rs#L173
    PLUGIN_RPC.stderr("Starting Tabnine LSP");
    PLUGIN_RPC.stderr(&format!(
        "Lsp path: {},\n Server args:{:#?}\n document_selector: {:#?}\n InitializeParams: {:#?}",
        server_path, server_args, document_selector, params.initialization_options
    ));
    PLUGIN_RPC.start_lsp(
        server_path,
        server_args,
        document_selector,
        params.initialization_options,
    );

    Ok(())
}

impl LapcePlugin for State {
    fn handle_request(&mut self, _id: u64, method: String, params: Value) {
        #[allow(clippy::single_match)]
        match method.as_str() {
            Initialize::METHOD => {
                let params: InitializeParams = serde_json::from_value(params).unwrap();
                if let Err(e) = initialize(params) {
                    PLUGIN_RPC.window_show_message(
                        MessageType::ERROR,
                        format!("plugin returned with error: {e}"),
                    )
                }
            }
            _ => {}
        }
    }
}
