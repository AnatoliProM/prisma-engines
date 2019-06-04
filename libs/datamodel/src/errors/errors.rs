use crate::ast::Span;
use colored::Colorize;

// No format for this file, on purpose.
// Line breaks make the declarations very hard to read.
#[cfg_attr(rustfmt, rustfmt_skip)] 

/// Enum for different errors which can happen during
/// parsing or validation.
/// 
/// For fancy printing, please use the `pretty_print_error` function.
#[derive(Debug, Fail, Clone, PartialEq)]
pub enum ValidationError {
    #[fail(display = "Argument {} is missing.", argument_name)]
    ArgumentNotFound { argument_name: String, span: Span },

    #[fail(display = "Function {} takes {} arguments, received {}", function_name, required_count, given_count)]
    ArgumentCountMissmatch { function_name: String, required_count: usize, given_count: usize, span: Span },

    #[fail(display = "Argument {} is missing in directive @{}.", argument_name, directive_name)]
    DirectiveArgumentNotFound { argument_name: String, directive_name: String, span: Span },

    #[fail(display = "Argument {} is missing in source block {}", argument_name, source_name)]
    SourceArgumentNotFound { argument_name: String, source_name: String, span: Span },

    #[fail(display = "Error parsing directive @{}: {}", directive_name, message)]
    DirectiveValidationError { message: String, directive_name: String, span: Span },

    #[fail(display = "Directive not known: @{}", directive_name)]
    DirectiveNotKnownError { directive_name: String, span: Span },

    #[fail(display = "Function not known: {}", function_name)]
    FunctionNotKnownError { function_name: String, span: Span },

    #[fail(display = "Source not known: {}", source_name)]
    SourceNotKnownError { source_name: String, span: Span },

    #[fail(display = "{} is not a valid value for {}.", raw_value, literal_type)]
    LiteralParseError { literal_type: String, raw_value: String, span: Span },

    #[fail(display = "Type {} is neither a built-in type, nor refers to another model or enum.", type_name)]
    TypeNotFoundError { type_name: String, span: Span },

    #[fail(display = "Type {} is not a built-in type.", type_name)]
    ScalarTypeNotFoundError { type_name: String, span: Span },

    #[fail(display = "Unexpected token. Expected one of: {}.", expected_str)]
    ParserError { expected: Vec<&'static str>, expected_str: String, span: Span },

    #[fail(display = "{}", message)]
    FunctionalEvaluationError { message: String, span: Span },

    #[fail(display = "Expected {}, but received {} value {}", expected_type, received_type, raw)]
    TypeMismatchError { expected_type: String, received_type: String, raw: String, span: Span },

    #[fail(display = "Expected {}, but failed while parsing {}: {}", expected_type, raw, parser_error)]
    ValueParserError { expected_type: String, parser_error: String, raw: String, span: Span },

    #[fail(display = "Error validating {}: {}", model_name, message)]
    ModelValidationError { message: String, model_name: String, span: Span  },

    #[fail(display = "Error validating: {}", message)]
    ValidationError { message: String, span: Span  },
}

#[cfg_attr(rustfmt, rustfmt_skip)] 
impl ValidationError {
    pub fn new_literal_parser_error(literal_type: &str, raw_value: &str, span: &Span) -> ValidationError {
        return ValidationError::LiteralParseError {
            literal_type: String::from(literal_type),
            raw_value: String::from(raw_value),
            span: span.clone(),
        };
    }

    pub fn new_argument_not_found_error(argument_name: &str, span: &Span) -> ValidationError {
        return ValidationError::ArgumentNotFound { argument_name: String::from(argument_name), span: span.clone() };
    }

    pub fn new_argument_count_missmatch_error(function_name: &str, required_count: usize, given_count: usize, span: &Span) -> ValidationError {
        return ValidationError::ArgumentCountMissmatch {
            function_name: String::from(function_name),
            required_count: required_count,
            given_count: given_count,
            span: span.clone(),
        };
    }

    pub fn new_directive_argument_not_found_error(argument_name: &str, directive_name: &str, span: &Span) -> ValidationError {
        return ValidationError::DirectiveArgumentNotFound {
            argument_name: String::from(argument_name),
            directive_name: String::from(directive_name),
            span: span.clone(),
        };
    }

    pub fn new_source_argument_not_found_error(argument_name: &str, source_name: &str, span: &Span) -> ValidationError {
        return ValidationError::SourceArgumentNotFound {
            argument_name: String::from(argument_name),
            source_name: String::from(source_name),
            span: span.clone(),
        };
    }

    pub fn new_directive_validation_error(message: &str, directive_name: &str, span: &Span) -> ValidationError {
        return ValidationError::DirectiveValidationError {
            message: String::from(message),
            directive_name: String::from(directive_name),
            span: span.clone(),
        };
    }

    pub fn new_model_validation_error(message: &str, model_name: &str, span: &Span) -> ValidationError {
        return ValidationError::ModelValidationError {
            message: String::from(message),
            model_name: String::from(model_name),
            span: span.clone(),
        };
    }

    pub fn new_validation_error(message: &str, span: &Span) -> ValidationError {
        return ValidationError::ValidationError {
            message: String::from(message),
            span: span.clone(),
        };
    }

    pub fn new_parser_error(expected: &Vec<&'static str>, span: &Span) -> ValidationError {
        return ValidationError::ParserError { expected: expected.clone(), expected_str: expected.join(", "), span: span.clone() };
    }
    pub fn new_functional_evaluation_error(message: &str, span: &Span) -> ValidationError {
        return ValidationError::FunctionalEvaluationError { message: String::from(message), span: span.clone() };
    }
    pub fn new_type_not_found_error(type_name: &str, span: &Span) -> ValidationError {
        return ValidationError::TypeNotFoundError { type_name: String::from(type_name), span: span.clone() };
    }
    pub fn new_scalar_type_not_found_error(type_name: &str, span: &Span) -> ValidationError {
        return ValidationError::ScalarTypeNotFoundError { type_name: String::from(type_name), span: span.clone() };
    }
    pub fn new_directive_not_known_error(directive_name: &str, span: &Span) -> ValidationError {
        return ValidationError::DirectiveNotKnownError { directive_name: String::from(directive_name), span: span.clone() };
    }
    pub fn new_function_not_known_error(function_name: &str, span: &Span) -> ValidationError {
        return ValidationError::FunctionNotKnownError { function_name: String::from(function_name), span: span.clone() };
    }

    pub fn new_source_not_known_error(source_name: &str, span: &Span) -> ValidationError {
        return ValidationError::SourceNotKnownError { source_name: String::from(source_name), span: span.clone() };
    }

    pub fn new_value_parser_error(expected_type: &str, parser_error: &str, raw: &str, span: &Span) -> ValidationError {
        return ValidationError::ValueParserError {
            expected_type: String::from(expected_type),
            parser_error: String::from(parser_error),
            raw: String::from(raw),
            span: span.clone(),
        };
    }

    pub fn new_type_mismatch_error(expected_type: &str, received_type: &str, raw: &str, span: &Span) -> ValidationError {
        return ValidationError::TypeMismatchError {
            expected_type: String::from(expected_type),
            received_type: String::from(received_type),
            raw: String::from(raw),
            span: span.clone(),
        };
    }

    pub fn span(&self) -> &Span {
        match self {
            ValidationError::ArgumentNotFound { argument_name: _, span } => span,
            ValidationError::DirectiveArgumentNotFound { argument_name: _, directive_name: _, span } => span,
            ValidationError::ArgumentCountMissmatch { function_name: _, required_count: _, given_count: _, span } => span,
            ValidationError::SourceArgumentNotFound { argument_name: _, source_name: _, span } => span,
            ValidationError::DirectiveValidationError { message: _, directive_name: _, span } => span,
            ValidationError::DirectiveNotKnownError { directive_name: _, span } => span,
            ValidationError::FunctionNotKnownError { function_name: _, span } => span,
            ValidationError::SourceNotKnownError { source_name: _, span } => span,
            ValidationError::LiteralParseError { literal_type: _, raw_value: _, span } => span,
            ValidationError::TypeNotFoundError { type_name: _, span } => span,
            ValidationError::ScalarTypeNotFoundError { type_name: _, span } => span,
            ValidationError::ParserError { expected: _, expected_str: _, span } => span,
            ValidationError::FunctionalEvaluationError { message: _, span } => span,
            ValidationError::TypeMismatchError { expected_type: _, received_type: _, raw: _, span } => span,
            ValidationError::ValueParserError { expected_type: _, parser_error: _, raw: _, span } => span,
            ValidationError::ValidationError { message: _, span } => span,
            ValidationError::ModelValidationError { model_name: _, message: _, span } => span
        }
    }
    pub fn description(&self) -> String {
        format!("{}", self)
    }

    pub fn pretty_print(&self, f: &mut std::io::Write, file_name: &str, text: &str) -> std::io::Result<()> {
        return pretty_print_error(f, file_name, text, self);
    }
}

/// Given the datamodel text representation, pretty prints an error, including
/// the offending portion of the source code, for human-friendly reading.
#[cfg_attr(rustfmt, rustfmt_skip)]
pub fn pretty_print_error(f: &mut std::io::Write, file_name: &str, text: &str, error_obj: &ValidationError) -> std::io::Result<()> {
    let span = error_obj.span();
    let error = error_obj.description();

    let line_number = text[..span.start].matches("\n").count();
    let file_lines = text.split("\n").collect::<Vec<&str>>();

    let chars_in_line_before: usize = file_lines[..line_number].iter().map(|l| l.len()).sum();
    // Don't forget to count the all the line breaks.
    let chars_in_line_before = chars_in_line_before + line_number;

    let line = &file_lines[line_number];

    let start_in_line = span.start - chars_in_line_before;
    let end_in_line = std::cmp::min(start_in_line + (span.end - span.start), line.len());

    let prefix = &line[..start_in_line];
    let offending = &line[start_in_line..end_in_line].bright_red().bold();
    let suffix = &line[end_in_line..];

    let arrow = "-->".bright_blue().bold();

    writeln!(f, "{}: {}", "error".bright_red().bold(), error.bold())?;
    writeln!(f, "  {}  {}:{}", arrow, file_name, line_number + 1)?;
    writeln!(f, "{}", format_line_number(0))?;
    writeln!(f, "{}", format_line_number_with_line(line_number, &file_lines))?;
    writeln!(f, "{}{}{}{}", format_line_number(line_number + 1), prefix, offending, suffix)?;
    if offending.len() == 0 {
        let spacing = std::iter::repeat(" ").take(start_in_line).collect::<String>();
        writeln!(f, "{}{}{}", format_line_number(0), spacing, "^ Unexpected token.".bold().bright_red())?;
    }
    writeln!(f, "{}", format_line_number_with_line(line_number + 2, &file_lines))?;
    writeln!(f, "{}", format_line_number(0))
}

fn format_line_number_with_line(line_number: usize, lines: &Vec<&str>) -> colored::ColoredString {
    if line_number > 0 && line_number <= lines.len() {
        colored::ColoredString::from(format!("{}{}", format_line_number(line_number), lines[line_number - 1]).as_str())
    } else {
        format_line_number(line_number)
    }
}
fn format_line_number(line_number: usize) -> colored::ColoredString {
    if line_number > 0 {
        format!("{:2} | ", line_number).bold().bright_blue()
    } else {
        "   | ".bold().bright_blue()
    }
}
