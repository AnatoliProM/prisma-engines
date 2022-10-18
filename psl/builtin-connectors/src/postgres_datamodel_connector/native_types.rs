crate::native_type_definition! {
    PostgresType;
    SmallInt -> Int,
    Integer -> Int,
    BigInt -> BigInt,
    Decimal(Option<(u32, u32)>) -> Decimal,
    Money -> Decimal,
    Inet -> String,
    Oid -> Int,
    Citext -> String,
    Real -> Float,
    DoublePrecision -> Float,
    VarChar(Option<u32>) -> String,
    Char(Option<u32>) -> String,
    Text -> String,
    ByteA -> Bytes,
    Timestamp(Option<u32>) -> DateTime,
    Timestamptz(Option<u32>) -> DateTime,
    Date -> DateTime,
    Time(Option<u32>) -> DateTime,
    Timetz(Option<u32>) -> DateTime,
    Boolean -> Boolean,
    Bit(Option<u32>) -> String,
    VarBit(Option<u32>) -> String,
    Uuid -> String,
    Xml -> String,
    Json -> Json,
    JsonB -> Json,
}
