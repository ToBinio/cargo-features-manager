use console::style;

pub fn highlight_search(text: &str, highlighted_letters: &Vec<usize>, is_dark: bool) -> String {
    text.chars()
        .enumerate()
        .map(|(index, c)| {
            match (is_dark, highlighted_letters.contains(&index)) {
                (false, true) => style(c).red().to_string(),
                (false, false) => c.to_string(),
                //dark red
                (true, true) => style(c).color256(1).to_string(),
                //light gray
                (true, false) => style(c).color256(8).to_string(),
            }
        })
        .collect()
}
