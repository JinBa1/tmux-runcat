use crate::tmux::Tmux;
use std::io;

const INSERT: &str = "#[nolist align=left default]#{E:@runcat-track}";
const RIGHT: &str = "#[nolist align=right";

fn format() -> String {
    let left = "#{w:#{T;=/#{status-left-length}:status-left}}";
    let right = "#{w:#{T;=/#{status-right-length}:status-right}}";
    let windows = "#{w:#{W:#{T:window-status-format}#{?loop_last_flag,,#{window-status-separator}},#{T:window-status-current-format}#{?loop_last_flag,,#{window-status-separator}}}}";
    let available =
        format!("#{{e|-:#{{client_width}},#{{e|+:{left},#{{e|+:{windows},{right}}}}}}}");
    let room = format!("#{{e|-:{available},#{{e|+:#{{w:#{{E:@runcat-roam-frame}}}},2}}}}");
    let offset = format!("#{{e|/:#{{e|*:#{{@runcat-position}},{room}}},1000}}");
    format!(
        "#{{?#{{>=:{room},0}}, #{{R: ,{offset}}}#{{E:@runcat-roam-frame}}#{{R: ,#{{e|-:{room},{offset}}}}} ,}}"
    )
}

pub fn sync(tmux: &Tmux, current: &str, enabled: bool) -> io::Result<()> {
    if enabled && !current.contains(INSERT) && current.contains(RIGHT) {
        let updated = current.replacen(RIGHT, &format!("{INSERT}{RIGHT}"), 1);
        tmux.execute([
            "set-option",
            "-g",
            "@runcat-track",
            &format(),
            ";",
            "set-option",
            "-g",
            "status-format[0]",
            &updated,
        ])?;
    } else if !enabled && current.contains(INSERT) {
        tmux.execute([
            "set-option",
            "-g",
            "status-format[0]",
            &current.replace(INSERT, ""),
        ])?;
    }
    Ok(())
}
