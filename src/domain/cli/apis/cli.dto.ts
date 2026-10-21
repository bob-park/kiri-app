/** Mirrors `src-tauri/src/app/cli_install.rs` `CliStatus`. */
export type CliStatus = {
  installed: boolean;
  link: string;
  target: string;
  socket_error: string | null;
};
