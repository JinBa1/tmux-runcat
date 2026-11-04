use crate::tmux::Tmux;
use std::io;

const INSERT: &str = "#[nolist align=left default]#{E:@runcat-track}";
const RIGHT: &str = "#[nolist align=right";
pub fn client_format(mut clients: Vec<u32>) -> String {
    clients.sort_unstable();
    clients.dedup();
    clients.into_iter().rev().fold(String::new(), |rest, pid| {
        format!("#{{?#{{==:#{{client_pid}},{pid}}},#{{E:@runcat-client-{pid}}},{rest}}}")
    })
}

/// Measured in the requesting client's format context, using rendered widths.
pub fn room_format() -> String {
    let left = "#{w:#{T;=/#{status-left-length}:status-left}}";
    let right = "#{w:#{T;=/#{status-right-length}:status-right}}";
    let windows = "#{w:#{W:#{T:window-status-format}#{?loop_last_flag,,#{window-status-separator}},#{T:window-status-current-format}#{?loop_last_flag,,#{window-status-separator}}}}";
    let occupied = format!("#{{e|+:{left},#{{e|+:{windows},{right}}}}}");
    let available = format!("#{{e|-:#{{client_width}},{occupied}}}");
    format!("#{{e|-:{available},#{{e|+:#{{w:#{{@runcat-frame}}}},2}}}}")
}

fn valid_color(value: &str) -> bool {
    if value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|c| c.is_ascii_hexdigit())
    {
        return true;
    }
    if let Some(number) = value
        .strip_prefix("colour")
        .or_else(|| value.strip_prefix("color"))
    {
        return number.parse::<u8>().is_ok();
    }
    matches!(
        value,
        "default"
            | "black"
            | "red"
            | "green"
            | "yellow"
            | "blue"
            | "magenta"
            | "cyan"
            | "white"
            | "brightblack"
            | "brightred"
            | "brightgreen"
            | "brightyellow"
            | "brightblue"
            | "brightmagenta"
            | "brightcyan"
            | "brightwhite"
    )
}

pub fn render(room: i32, position: usize, pose: &str, color: &str) -> String {
    if room < 0 {
        return String::new();
    }
    let room = room as usize;
    let position = position.min(room);
    let color = color.trim();
    let style = if valid_color(color) {
        format!("#[fg={color}]")
    } else {
        String::new()
    };
    format!(
        " {}{style}{pose}#[default]{} ",
        " ".repeat(position),
        " ".repeat(room - position)
    )
}

pub fn sync(tmux: &Tmux, current: &str, enabled: bool, format: &str) -> io::Result<()> {
    if enabled && current.contains(RIGHT) {
        // Also migrate an already installed track from an older worker.
        tmux.execute(["set-option", "-g", "@runcat-track", format])?;
        if !current.contains(INSERT) {
            let updated = current.replacen(RIGHT, &format!("{INSERT}{RIGHT}"), 1);
            tmux.execute(["set-option", "-g", "status-format[0]", &updated])?;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colors_are_literal_and_scoped_to_the_cat() {
        for color in ["cyan", "#12aBcD", "colour123", "default"] {
            assert_eq!(
                render(3, 1, "CAT", color),
                format!("  #[fg={color}]CAT#[default]   ")
            );
        }
        for color in ["inherit", "", "colour256", "cyan]#[bg=red", "#{@other}"] {
            assert_eq!(render(3, 1, "CAT", color), "  CAT#[default]   ");
        }
        assert_eq!(render(-1, 50, "CAT", "cyan"), "");
    }
}
