use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

use llm_wiki_app_core::{ProviderId, ProviderStatus, ProviderSummary, ProviderTransport};

pub fn detect_all() -> Vec<ProviderSummary> {
    vec![
        detect_codex(),
        detect_cli(
            ProviderId::Claude,
            "Claude",
            "claude.exe",
            "claude",
            vec!["install", "login", "models", "structured_output"],
        ),
        detect_cli(
            ProviderId::Antigravity,
            "Antigravity",
            "agy.exe",
            "agy",
            vec!["install", "login", "models", "structured_output"],
        ),
        ProviderSummary {
            provider_id: ProviderId::Openrouter,
            display_name: "OpenRouter".to_owned(),
            transport: ProviderTransport::CloudApi,
            status: ProviderStatus::KeyRequired,
            version: None,
            selected_model: None,
            detail: Some("provider.openrouter.key_required".to_owned()),
            capabilities: strings(["credentials", "models", "structured_output"]),
        },
        detect_ollama(),
    ]
}

fn detect_codex() -> ProviderSummary {
    let Some(path) = find_executable("codex.exe", "codex") else {
        return missing_cli(
            ProviderId::Codex,
            "Codex",
            vec!["install", "login", "models", "structured_output"],
        );
    };
    let version = read_version(&path);
    let authenticated = command_succeeds(&path, &["login", "status"]);
    ProviderSummary {
        provider_id: ProviderId::Codex,
        display_name: "Codex".to_owned(),
        transport: ProviderTransport::Cli,
        status: if authenticated {
            ProviderStatus::Connected
        } else {
            ProviderStatus::AuthRequired
        },
        version,
        selected_model: None,
        detail: Some(
            if authenticated {
                "provider.codex.connected"
            } else {
                "provider.codex.auth_required"
            }
            .to_owned(),
        ),
        capabilities: strings(["install", "login", "models", "structured_output"]),
    }
}

fn detect_cli(
    provider_id: ProviderId,
    display_name: &str,
    windows_name: &str,
    fallback_name: &str,
    capabilities: Vec<&str>,
) -> ProviderSummary {
    let Some(path) = find_executable(windows_name, fallback_name) else {
        return missing_cli(provider_id, display_name, capabilities);
    };
    ProviderSummary {
        provider_id,
        display_name: display_name.to_owned(),
        transport: ProviderTransport::Cli,
        status: ProviderStatus::AuthRequired,
        version: read_version(&path),
        selected_model: None,
        detail: Some("provider.auth_status_requires_setup".to_owned()),
        capabilities: capabilities.into_iter().map(str::to_owned).collect(),
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

fn detect_ollama() -> ProviderSummary {
    let installed = find_executable("ollama.exe", "ollama");
    let online = TcpStream::connect_timeout(
        &SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 11_434),
        Duration::from_millis(180),
    )
    .is_ok();
    let (status, detail) = if online {
        (ProviderStatus::Connected, "provider.ollama.connected")
    } else if installed.is_some() {
        (
            ProviderStatus::InstalledOffline,
            "provider.ollama.installed_offline",
        )
    } else {
        (
            ProviderStatus::NotInstalled,
            "provider.ollama.not_installed",
        )
    };
    ProviderSummary {
        provider_id: ProviderId::Ollama,
        display_name: "Ollama".to_owned(),
        transport: ProviderTransport::LocalHttp,
        status,
        version: installed.as_ref().and_then(|path| read_version(path)),
        selected_model: None,
        detail: Some(detail.to_owned()),
        capabilities: strings(["install", "models", "model_pull", "structured_output"]),
    }
}

fn find_executable(windows_name: &str, fallback_name: &str) -> Option<PathBuf> {
    [windows_name, fallback_name].into_iter().find_map(|name| {
        let output = hidden_command("where.exe")
            .arg(name)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(PathBuf::from)
    })
}

fn read_version(path: &PathBuf) -> Option<String> {
    let output = hidden_command(path)
        .arg("--version")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!version.is_empty()).then_some(version)
}

fn command_succeeds(path: &PathBuf, args: &[&str]) -> bool {
    hidden_command(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn strings<const N: usize>(values: [&str; N]) -> Vec<String> {
    values.into_iter().map(str::to_owned).collect()
}

fn hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::detect_all;

    #[test]
    fn reports_every_production_provider_once() {
        let providers = detect_all();
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
