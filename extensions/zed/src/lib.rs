//! Tailor as a Zed extension.
//!
//! Zed extensions are WebAssembly and cannot draw anything — the capability
//! list is languages, debuggers, themes, icon themes, snippets and MCP servers.
//! So this is not a Tailor panel inside Zed, and could not be. It is the seam
//! Zed does offer: it registers `tailor-mcp` as a context server, so the agent
//! panel can build and edit a `.tailor` document without leaving the editor.
//!
//! The other half of the integration needs nothing from Zed at all. The server
//! saves after every change and Tailor watches the file it has open, so a
//! screen built from here appears on the canvas a moment later; and Tailor's
//! *Open in Editor* shells the `zed` CLI at the generated line.
//!
//! Note that the designer<->editor jump loop is a separate thing from this
//! extension and needs none of it: see `docs/tailorzed.md`.

use zed_extension_api::{
    self as zed, settings::ContextServerSettings, Command, ContextServerId, Project, Result,
};

/// Where the app bundle puts the server. Tailor ships it beside the executable
/// rather than expecting a separate install, so this is the path that works for
/// anyone who dragged the DMG to Applications.
const BUNDLED: &str = "/Applications/Tailor.app/Contents/MacOS/tailor-mcp";

/// What to fall back to: a development build, or anything else on `$PATH`.
const ON_PATH: &str = "tailor-mcp";

struct TailorExtension;

impl zed::Extension for TailorExtension {
    fn new() -> Self {
        TailorExtension
    }

    fn context_server_command(
        &mut self,
        context_server_id: &ContextServerId,
        project: &Project,
    ) -> Result<Command> {
        // A configured path wins, always. `context_server_command` is handed a
        // project and not a worktree, so there is no `which` to ask here — a
        // user whose binary is somewhere unusual has settings and nothing else.
        let configured = ContextServerSettings::for_project(context_server_id.as_ref(), project)
            .ok()
            .and_then(|settings| settings.command);

        if let Some(command) = configured {
            if let Some(path) = command.path.filter(|path| !path.is_empty()) {
                return Ok(Command {
                    command: path,
                    args: command.arguments.unwrap_or_default(),
                    env: command.env.unwrap_or_default().into_iter().collect(),
                });
            }
        }

        // Otherwise the bundled server if it is there, and the bare name if it
        // is not — Zed resolves that against `$PATH` when it spawns.
        let command = if std::path::Path::new(BUNDLED).exists() {
            BUNDLED.to_string()
        } else {
            ON_PATH.to_string()
        };

        Ok(Command {
            command,
            args: Vec::new(),
            env: Vec::new(),
        })
    }
}

zed::register_extension!(TailorExtension);
