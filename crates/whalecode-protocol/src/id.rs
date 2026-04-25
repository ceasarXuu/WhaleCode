use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(pub String);

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_owned())
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }
    };
}

id_type!(AgentId);
id_type!(ArtifactId);
id_type!(GateId);
id_type!(PrimitiveId);
id_type!(SessionId);
id_type!(ToolCallId);
id_type!(TraceId);
id_type!(TurnId);
id_type!(WorkUnitId);
