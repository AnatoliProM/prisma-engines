use crate::dml::validator::directive::{Args, DirectiveValidator, Error};
use crate::{ast, dml};

/// Prismas builtin `@db` directive.
pub struct DbDirectiveValidator {}

impl<T: dml::WithDatabaseName> DirectiveValidator<T> for DbDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"db"
    }
    fn validate_and_apply(&self, args: &Args, obj: &mut T) -> Result<(), Error> {
        match args.default_arg("name")?.as_str() {
            Ok(value) => obj.set_database_name(&Some(value)),
            // self.parser_error would be better here, but we cannot call it due to rust limitations.
            Err(err) => {
                return Err(Error::new_directive_validation_error(
                    &format!("{}", err),
                    "db",
                    &err.span(),
                ))
            }
        };

        return Ok(());
    }

    fn serialize(&self, obj: &T) -> Result<Option<ast::Directive>, Error> {
        if let Some(db_name) = obj.database_name() {
            return Ok(Some(ast::Directive::new(
                DirectiveValidator::<T>::directive_name(self),
                vec![ast::Argument::new_string("", db_name)],
            )));
        }

        Ok(None)
    }
}
