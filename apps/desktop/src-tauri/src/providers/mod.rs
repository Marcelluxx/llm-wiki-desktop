use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::PathBuf,
    time::Duration,
};

use llm_wiki_app_core::{ProviderId, ProviderStatus, ProviderSummary, ProviderTransport};

/// Startup-safe detection. This only inspects PATH and known install folders;
/// it never starts a provider CLI, so opening the app stays instantaneous.
pub fn detect_all_fast() -> Vec<ProviderSummary> {
    let openrouter_configured = crate::openrouter_credential_exists();
    let openrouter_model = crate::openrouter_selected_model();
    vec![
        detect_cli_fast(ProviderId::Codex, "Codex", "codex"),
        detect_cli_fast(ProviderId::Claude, "Claude", "claude"),
        detect_cli_fast(ProviderId::Antigravity, "Antigravity", "agy"),
        ProviderSummary {
            provider_id: ProviderId::Openrouter,
            display_name: "OpenRouter".to_owned(),
            transport: ProviderTransport::CloudApi,
            status: if openrouter_configured && openrouter_model.is_some() {
                ProviderStatus::Connected
            } else if openrouter_configured {
                ProviderStatus::ActionRequired
            } else {
                ProviderStatus::KeyRequired
            },
            version: None,
            selected_model: openrouter_model,
            detail: Some(
                if openrouter_configured {
                    "provider.openrouter.connected"
                } else {
                    "provider.openrouter.key_required"
                }
                .to_owned(),
            ),
            capabilities: strings(["credentials", "models", "structured_output"]),
        },
        detect_ollama_fast(),
    ]
}

fn detect_cli_fast(provider_id: ProviderId, display_name: &str, name: &str) -> ProviderSummary {
    if find_executable_fast(name).is_none() {
        return missing_cli(
            provider_id,
            display_name,
            vec!["install", "login", "models", "structured_output"],
        );
    }
    let authenticated = crate::provider_auth_marker_exists(provider_id);
    ProviderSummary {
        provider_id,
        display_name: display_name.to_owned(),
        transport: ProviderTransport::Cli,
        status: if authenticated {
            ProviderStatus::Connected
        } else {
            ProviderStatus::AuthRequired
        },
        version: None,
        selected_model: None,
        detail: Some("provider.auth_status_requires_setup".to_owned()),
        capabilities: strings(["install", "login", "models", "structured_output"]),
    }
}

fn detect_ollama_fast() -> ProviderSummary {
    let installed = find_executable_fast("ollama").is_some();
    let online = TcpStream::connect_timeout(
        &SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 11_434),
        Duration::from_millis(60),
    )
    .is_ok();
    ProviderSummary {
        provider_id: ProviderId::Ollama,
        display_name: "Ollama".to_owned(),
        transport: ProviderTransport::LocalHttp,
        status: if online {
            ProviderStatus::Connected
        } else if installed {
            ProviderStatus::InstalledOffline
        } else {
            ProviderStatus::NotInstalled
        },
        version: None,
        selected_model: crate::ollama_selected_model(),
        detail: None,
        capabilities: strings(["install", "models", "model_pull", "structured_output"]),
    }
}

fn missing_cli(
    provider_id: ProviderId,
    display_name: &str,
    capabilities: Vec<&str>,
) -> ProviderSummary {
    ProviderSummary {
        provider_id,
        display_name: display_name.to_owned(),
        transport: ProviderTransport::Cli,
        status: ProviderStatus::NotInstalled,
        version: None,
        selected_model: None,
        detail: Some("provider.cli.not_installed".to_owned()),
        capabilities: capabilities.into_iter().map(str::to_owned).collect(),
    }
}

pub(crate) fn find_executable_fast(name: &str) -> Option<PathBuf> {
    let extensions = ["exe", "cmd", "bat"];
    let from_path = std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path).find_map(|folder| {
            extensions.iter().find_map(|extension| {
                let candidate = folder.join(format!("{name}.{extension}"));
                candidate.is_file().then_some(candidate)
            })
        })
    });
    if from_path.is_some() {
        return from_path;
    }

    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let roaming = std::env::var_os("APPDATA").map(PathBuf::from);
    let known = match name {
        "agy" => local.map(|root| root.join("agy/bin/agy.exe")),
        "ollama" => local.map(|root| root.join("Programs/Ollama/ollama.exe")),
        "codex" => roaming.map(|root| root.join("npm/codex.cmd")).or_else(|| {
            let versions = local?.join("OpenAI/Codex/bin");
            std::fs::read_dir(versions).ok()?.find_map(|entry| {
                let candidate = entry.ok()?.path().join("codex.exe");
                candidate.is_file().then_some(candidate)
            })
        }),
        "claude" => roaming
            .map(|root| root.join("npm/claude.cmd"))
            .filter(|path| path.is_file())
            .or_else(|| {
                local.map(|root| {
                    root.join("LLMWiki/runtime/providers/claude/node_modules/.bin/claude.cmd")
                })
            }),
        _ => None,
    };
    known.filter(|path| path.is_file())
}

fn strings<const N: usize>(values: [&str; N]) -> Vec<String> {
    values.into_iter().map(str::to_owned).collect()
}

#[cfg(test)]
mod tests {
    use super::detect_all_fast;

    #[test]
    fn reports_every_production_provider_once() {
        let providers = detect_all_fast();
        assert_eq!(providers.len(), 5);
        let mut ids = providers
            .iter()
            .map(|provider| format!("{:?}", provider.provider_id))
            .collect::<Vec<_>>();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 5);
    }
}
