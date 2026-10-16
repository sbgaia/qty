//! `snake_case` from UnitsNet's `PascalCase` names, with the word rules of
//! the `heck` crate: a word ends before an uppercase letter that follows a
//! lowercase one (`KiloMeter` → `kilo_meter`), and before the last capital
//! of a run followed by a lowercase letter (`SIUnit` → `si_unit`); digits
//! stay with the letters before them.

pub trait ToSnakeCase {
    fn to_snake_case(&self) -> String;
}

impl ToSnakeCase for str {
    fn to_snake_case(&self) -> String {
        #[derive(Clone, Copy, PartialEq)]
        enum Mode {
            Boundary,
            Lower,
            Upper,
        }
        let mut words: Vec<&str> = Vec::new();
        for part in self.split(|c: char| !c.is_alphanumeric()) {
            let mut chars = part.char_indices().peekable();
            let mut start = 0;
            let mut mode = Mode::Boundary;
            while let Some((i, c)) = chars.next() {
                let Some(&(next_i, next)) = chars.peek() else {
                    words.push(&part[start..]);
                    break;
                };
                let next_mode = if c.is_lowercase() {
                    Mode::Lower
                } else if c.is_uppercase() {
                    Mode::Upper
                } else {
                    mode
                };
                if next_mode == Mode::Lower && next.is_uppercase() {
                    words.push(&part[start..next_i]);
                    start = next_i;
                    mode = Mode::Boundary;
                } else if mode == Mode::Upper && c.is_uppercase() && next.is_lowercase() {
                    words.push(&part[start..i]);
                    start = i;
                    mode = Mode::Boundary;
                } else {
                    mode = next_mode;
                }
            }
        }
        let words: Vec<String> = words
            .into_iter()
            .filter(|w| !w.is_empty())
            .map(str::to_lowercase)
            .collect();
        words.join("_")
    }
}

#[cfg(test)]
mod tests {
    use super::ToSnakeCase;

    #[test]
    fn words() {
        assert_eq!("Meter".to_snake_case(), "meter");
        assert_eq!("KilometersPerHour".to_snake_case(), "kilometers_per_hour");
        assert_eq!("BtuPerHour".to_snake_case(), "btu_per_hour");
        assert_eq!("SIUnit".to_snake_case(), "si_unit");
        assert_eq!("Kilowatt2Hours".to_snake_case(), "kilowatt2_hours");
        assert_eq!("Sqrt".to_snake_case(), "sqrt");
        assert_eq!("already_snake".to_snake_case(), "already_snake");
    }
}
