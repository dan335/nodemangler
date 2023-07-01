use crate::{node_settings::NodeSettings, operations};
use serde::{Deserialize, Serialize};
use crate::{input::Input, output::Output};



operations! {
    OpNumberInputInteger(crate::operations::numbers::inputs::integer::OpNumberInputInteger),
    OpNumberInputDecimal(crate::operations::numbers::inputs::decimal::OpNumberInputDecimal),
}





