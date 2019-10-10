use crate::errors::DatamodelError;
use crate::validator::directive::{Args, DirectiveValidator};
use crate::{ast, dml};

/// Prismas builtin `@scalarList` directive.
pub struct ScalarListDirectiveValidator {}

impl DirectiveValidator<dml::Field> for ScalarListDirectiveValidator {
    fn directive_name(&self) -> &'static str {
        &"scalarList"
    }

    fn validate_and_apply(&self, args: &mut Args, obj: &mut dml::Field) -> Result<(), DatamodelError> {
        // TODO: Throw when field is not of type scalar and arity is list.
        // TODO: We can probably lift this pattern to a macro.

        match args.arg("strategy")?.parse_literal::<dml::ScalarListStrategy>() {
            Ok(strategy) => obj.scalar_list_strategy = Some(strategy),
            Err(err) => return Err(self.parser_error(&err)),
        }

        Ok(())
    }

    fn serialize(&self, obj: &dml::Field, _datamodel: &dml::Datamodel) -> Result<Vec<ast::Directive>, DatamodelError> {
        if let Some(strategy) = &obj.scalar_list_strategy {
            return Ok(vec![ast::Directive::new(
                self.directive_name(),
                vec![ast::Argument::new_constant("strategy", &strategy.to_string())],
            )]);
        }

        Ok(vec![])
    }
}
