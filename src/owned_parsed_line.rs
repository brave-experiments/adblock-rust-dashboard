use adblock::lists::{parse_filter, FilterParseError, ParsedLine, ParseOptions};

/// ParsedLine is only valid for the lifetime of the parsed string input. This wrapper safely
/// produces a `'static` version of the parse result, storing the line input alongside it.
pub struct OwnedParsedLine {
    line: String,
    parse_result: Result<ParsedLine<'static>, FilterParseError>,
}

impl Default for OwnedParsedLine {
    fn default() -> Self {
        Self {
            line: Default::default(),
            parse_result: Err(FilterParseError::Empty)
        }
    }
}

impl OwnedParsedLine {
    pub fn update_line(&mut self, line: &str, parse_options: ParseOptions) {
        let parse_result = parse_filter(line, true, parse_options);

        self.line = line.to_string();
        self.parse_result = unsafe {
            std::mem::transmute::<_, Result<ParsedLine<'static>, FilterParseError>>(parse_result)
        };
    }

    pub fn get<'a>(&'a self) -> &'a Result<ParsedLine<'a>, FilterParseError> {
        &self.parse_result
    }
}
