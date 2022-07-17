#[cfg(test)]
mod tests {
    use crate::db::*;
    use r2d2_sqlite::SqliteConnectionManager;
    use teloxide::types::UserId;

    #[test]
    fn test_db() -> Result<(), rusqlite::Error> {
        let db_file = "./test.db3";
        let _ = std::fs::remove_file(db_file);
        let manager = SqliteConnectionManager::file(db_file);
        let pool = r2d2::Pool::new(manager).unwrap();
        let conn = pool.get().unwrap();
        create(&conn).expect("Failed to create db.");

        let ts = 1650445814;

        let e = Event {
            id: 0,
            name: "test event 1".to_string(),
            link: "https://example.com/1".to_string(),
            max_adults: 2,
            max_children: 2,
            max_adults_per_reservation: 1,
            max_children_per_reservation: 3,
            ts: ts,
            remind: ts - 10,
        };
        let event_id = 1;

        // new event
        assert_eq!(add_event(&conn, e.clone()), Ok(1));

        // user 1000 reserves for two children and places one on the waiting list
        for i in 0..3 {
            assert_eq!(
                sign_up(
                    &conn,
                    event_id,
                    &User {
                        id: UserId(1000),
                        user_name1: "user_name1_1000".to_string(),
                        user_name2: "user_name2_1000".to_string(),
                        is_admin: false
                    },
                    0,
                    1,
                    match i == 2 {
                        true => 1,
                        false => 0,
                    },
                    e.ts - 20,
                )
                .unwrap(),
                (1, false)
            );
        }

        let s = get_event(&conn, event_id, 1000)?;
        assert_eq!(s.children.my_reservation, 2);
        assert_eq!(s.children.my_waiting, 1);

        // user 2000 places one child on the waiting list
        assert_eq!(
            sign_up(
                &conn,
                event_id,
                &User {
                    id: UserId(2000),
                    user_name1: "user_name1_2000".to_string(),
                    user_name2: "user_name1_2000".to_string(),
                    is_admin: false
                },
                0,
                1,
                1,
                e.ts - 20,
            )
            .unwrap(),
            (1, false)
        );

        let s = get_event(&conn, event_id, 2000).unwrap();
        assert_eq!(s.children.my_waiting, 1);

        let events = get_events(&conn, 0, 0, 20).unwrap();
        assert_eq!(events.len(), 1);

        // time for cleanup
        clear_old_events(
            &conn,
            ts + 20 * 60 * 60,
            false,
            false,
            &HashSet::<u64>::new(),
        )?;

        let events = get_events(&conn, 0, 0, 20).unwrap();
        assert_eq!(events.len(), 0);

        Ok(())
    }

    #[test]
    #[ignore]
    fn test_waiting_list() -> Result<(), rusqlite::Error> {
        let db_file = "./test1.db3";
        let _ = std::fs::remove_file(db_file);
        let manager = SqliteConnectionManager::file(db_file);
        let pool = r2d2::Pool::new(manager).unwrap();
        let conn = pool.get().unwrap();
        create(&conn).expect("Failed to create db.");

        let ts = 1650445814;

        let e = Event {
            id: 0,
            name: "test event 1".to_string(),
            link: "https://example.com/1".to_string(),
            max_adults: 1,
            max_children: 0,
            max_adults_per_reservation: 1,
            max_children_per_reservation: 3,
            ts: ts,
            remind: ts - 10,
        };
        let event_id = 1;

        // new event
        assert_eq!(add_event(&conn, e.clone()), Ok(1));

        // sign up
        assert_eq!(
            sign_up(
                &conn,
                event_id,
                &User {
                    id: UserId(10),
                    user_name1: "".to_string(),
                    user_name2: "".to_string(),
                    is_admin: false
                },
                1,
                0,
                0,
                e.ts - 30,
            )
            .unwrap(),
            (1, false)
        );

        // add to waiting list
        assert_eq!(
            sign_up(
                &conn,
                event_id,
                &User {
                    id: UserId(20),
                    user_name1: "".to_string(),
                    user_name2: "".to_string(),
                    is_admin: false
                },
                1,
                0,
                1,
                e.ts - 20,
            )
            .unwrap(),
            (1, false)
        );
        assert_eq!(
            sign_up(
                &conn,
                event_id,
                &User {
                    id: UserId(30),
                    user_name1: "".to_string(),
                    user_name2: "".to_string(),
                    is_admin: false
                },
                1,
                0,
                1,
                e.ts - 10,
            )
            .unwrap(),
            (1, false)
        );

        let s = get_event(&conn, event_id, 10)?;
        assert_eq!(s.adults.my_reservation, 1);

        let s = get_event(&conn, event_id, 20)?;
        assert_eq!(s.adults.my_reservation, 0);
        assert_eq!(s.adults.my_waiting, 1);

        let s = get_event(&conn, event_id, 30)?;
        assert_eq!(s.adults.my_reservation, 0);
        assert_eq!(s.adults.my_waiting, 1);

        cancel(&conn, event_id, 10, 1)?;

        let s = get_event(&conn, event_id, 10)?;
        assert_eq!(s.adults.my_reservation, 0);

        let s = get_event(&conn, event_id, 20)?;
        assert_eq!(s.adults.my_reservation, 1);
        assert_eq!(s.adults.my_waiting, 0);

        let s = get_event(&conn, event_id, 30)?;
        assert_eq!(s.adults.my_reservation, 0);
        assert_eq!(s.adults.my_waiting, 1);

        cancel(&conn, event_id, 20, 1)?;

        let s = get_event(&conn, event_id, 20)?;
        assert_eq!(s.adults.my_reservation, 0);

        let s = get_event(&conn, event_id, 30)?;
        assert_eq!(s.adults.my_reservation, 1);
        assert_eq!(s.adults.my_waiting, 0);

        cancel(&conn, event_id, 30, 1)?;

        let s = get_event(&conn, event_id, 10)?;
        assert_eq!(s.adults.my_reservation, 0);
        assert_eq!(s.adults.my_waiting, 0);

        let s = get_event(&conn, event_id, 20)?;
        assert_eq!(s.adults.my_reservation, 0);
        assert_eq!(s.adults.my_waiting, 0);

        let s = get_event(&conn, event_id, 30)?;
        assert_eq!(s.adults.my_reservation, 0);
        assert_eq!(s.adults.my_waiting, 0);

        Ok(())
    }
}
