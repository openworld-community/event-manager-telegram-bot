use crate::db::*;

#[test]
fn test() -> Result<(), rusqlite::Error> {
    let db_file = "./test.db3";
    let _ = std::fs::remove_file(db_file);
    let db = EventDB::open(db_file)
        .map_err(|e| format!("Failed to init db: {}", e.to_string()))
        .unwrap();

    let ts = 1650445814;

    let e = Event {
        id: 1,
        name: "test event 1".to_string(),
        link: "https://example.com/1".to_string(),
        max_adults: 2,
        max_children: 2,
        max_adults_per_reservation: 1,
        max_children_per_reservation: 3,
        ts: ts,
        remind: ts - 10,
    };

    // new event
    assert_eq!(db.add_event(e.clone()), Ok(1));

    // user 1000 reserves for two children and places one on the waiting list
    for i in 0..3 {
        assert_eq!(
            db.sign_up(
                e.id,
                1000,
                "user_name1_1000",
                "user_name2_1000",
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

    let s = db.get_event(e.id, 1000)?;
    assert_eq!(s.my_children, 2);
    assert_eq!(s.my_wait_children, 1);

    // user 2000 places one child on the waiting list
    assert_eq!(
        db.sign_up(
            e.id,
            2000,
            "user_name1_2000",
            "user_name2_2000",
            0,
            1,
            1,
            e.ts - 20,
        )
        .unwrap(),
        (1, false)
    );

    let s = db.get_event(e.id, 2000).unwrap();
    assert_eq!(s.my_wait_children, 1);

    let events = db.get_events(0).unwrap();
    assert_eq!(events.len(), 1);

    // time to send reminders
    let reminders = db.get_user_reminders(ts - 9).unwrap();
    assert_eq!(
        reminders[0],
        Reminder {
            event_id: e.id,
            name: e.name,
            link: e.link,
            ts: ts,
            user_id: 1000,
        }
    );

    db.clear_user_reminders(ts - 9)?;

    let reminders = db.get_user_reminders(ts - 9).unwrap();
    assert_eq!(reminders.len(), 0);

    // user 1000 cancels, user 2000 receives notification
    let notifications = db.wontgo(e.id, 1000).unwrap();
    assert!(notifications.contains(&2000));

    // user 2000 has a reservation now
    let s = db.get_event(e.id, 2000).unwrap();
    assert_eq!(s.my_children, 1);
    assert_eq!(s.my_wait_children, 0);

    // time for cleanup
    db.clear_old_events(ts + 20 * 60 * 60, false)?;

    let events = db.get_events(0).unwrap();
    assert_eq!(events.len(), 0);

    let _ = db.conn.close();
    Ok(())
}

#[test]
fn test_waiting_list() -> Result<(), rusqlite::Error> {
    let db_file = "./test1.db3";
    let _ = std::fs::remove_file(db_file);
    let db = EventDB::open(db_file)
        .map_err(|e| format!("Failed to init db: {}", e.to_string()))
        .unwrap();

    let ts = 1650445814;

    let e = Event {
        id: 1,
        name: "test event 1".to_string(),
        link: "https://example.com/1".to_string(),
        max_adults: 1,
        max_children: 0,
        max_adults_per_reservation: 1,
        max_children_per_reservation: 3,
        ts: ts,
        remind: ts - 10,
    };

    // new event
    assert_eq!(db.add_event(e.clone()), Ok(1));

    // sign up
    assert_eq!(
        db.sign_up(e.id, 10, "", "", 1, 0, 0, e.ts - 30,).unwrap(),
        (1, false)
    );

    // add to waiting list
    assert_eq!(
        db.sign_up(e.id, 20, "", "", 1, 0, 1, e.ts - 20,).unwrap(),
        (1, false)
    );
    assert_eq!(
        db.sign_up(e.id, 30, "", "", 1, 0, 1, e.ts - 10,).unwrap(),
        (1, false)
    );

    let s = db.get_event(e.id, 10)?;
    assert_eq!(s.my_adults, 1);

    let s = db.get_event(e.id, 20)?;
    assert_eq!(s.my_adults, 0);
    assert_eq!(s.my_wait_adults, 1);

    let s = db.get_event(e.id, 30)?;
    assert_eq!(s.my_adults, 0);
    assert_eq!(s.my_wait_adults, 1);

    db.cancel(e.id, 10, 1)?;

    let s = db.get_event(e.id, 10)?;
    assert_eq!(s.my_adults, 0);

    let s = db.get_event(e.id, 20)?;
    assert_eq!(s.my_adults, 1);
    assert_eq!(s.my_wait_adults, 0);

    let s = db.get_event(e.id, 30)?;
    assert_eq!(s.my_adults, 0);
    assert_eq!(s.my_wait_adults, 1);

    db.cancel(e.id, 20, 1)?;

    let s = db.get_event(e.id, 20)?;
    assert_eq!(s.my_adults, 0);

    let s = db.get_event(e.id, 30)?;
    assert_eq!(s.my_adults, 1);
    assert_eq!(s.my_wait_adults, 0);

    db.cancel(e.id, 30, 1)?;

    let s = db.get_event(e.id, 10)?;
    assert_eq!(s.my_adults, 0);
    assert_eq!(s.my_wait_adults, 0);

    let s = db.get_event(e.id, 20)?;
    assert_eq!(s.my_adults, 0);
    assert_eq!(s.my_wait_adults, 0);

    let s = db.get_event(e.id, 30)?;
    assert_eq!(s.my_adults, 0);
    assert_eq!(s.my_wait_adults, 0);

    let _ = db.conn.close();
    Ok(())
}
