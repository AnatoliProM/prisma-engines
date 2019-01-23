#[macro_use]
extern crate lazy_static;

use r2d2;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::NO_PARAMS;

lazy_static! {
    pub static ref POOL: r2d2::Pool<SqliteConnectionManager> = {
        let manager = SqliteConnectionManager::file("test.db");
        r2d2::Pool::new(manager).unwrap()
    };
}

#[no_mangle]
pub extern "C" fn select_1() -> i32 {
    let conn = POOL.get().unwrap();
    let mut stmt = conn.prepare("SELECT 1").unwrap();
    let mut rows = stmt.query_map(NO_PARAMS, |row| row.get(0)).unwrap();

    match rows.next() {
        Some(r) => r.unwrap(),
        None => panic!("No result"),
    }

    //.unwrap().unwrap() as i32
}

#[cfg(test)]
mod test {
    use rusqlite::NO_PARAMS;

    #[test]
    fn test_basic_select() {
        let conn = super::POOL.get().unwrap();
        let mut stmt = conn.prepare("SELECT 1").unwrap();

        let rows = stmt.query_map(NO_PARAMS, |row| row.get(0)).unwrap();

        for val in rows {
            let value: i32 = val.unwrap();

            assert_eq!(1, value);
        }
    }
}
