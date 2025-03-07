use std::fmt::Debug;

use async_std::task;
use worktable::prelude::*;
use worktable::worktable;

fn main() {
    // describe WorkTable
    worktable!(
        name: My,
        columns: {
            id: u64 primary_key autoincrement,
            val: i64,
            attribute: String,

        },
        indexes: {
            attribute_idx: attribute,
        }
        queries: {
            update: {
                ValByAttr(val) by attribute,
            },
            delete: {
                ByAttr() by attribute,
                ById() by id,
            }
        }
    );

    // Init Worktable
    let my_table = MyWorkTable::default();

    // WT rows (has prefix My because of table name)
    let row = MyRow {
        val: 1,
        attribute: "TEST".to_string(),
        id: 0,
    };

    let row1 = MyRow {
        val: 2,
        attribute: "TEST2".to_string(),
        id: 1,
    };

    let row2 = MyRow {
        val: 1337,
        attribute: "TEST2".to_string(),
        id: 2,
    };

    let row3 = MyRow {
        val: 555,
        attribute: "TEST3".to_string(),
        id: 3,
    };

    // insert
    let _ = my_table.insert(row);
    let _ = my_table.insert(row1);
    let _ = my_table.insert(row2);
    let _ = my_table.insert(row3);

    // Select ALL records from WT
    let select_all = my_table.select_all().execute();
    println!("Select All {:?}", select_all);

    // Select All records with attribute TEST2
    let select_by_attr = my_table.select_by_attribute("TEST2".to_string());
    println!(
        "Select by Attribute TEST2: {:?}",
        select_by_attr.unwrap().vals
    );

    // Update all recrods val by attr TEST2
    let update_val = my_table.update_val_by_attr(ValByAttrQuery { val: 777 }, "TEST2".to_string());
    let _ = task::block_on(update_val);

    let select_updated = my_table.select_by_attribute("TEST2".to_string());
    println!(
        "Select updated by Attribute TEST2: {:?}",
        select_updated.unwrap().vals
    );

    // Update record attribute TEST2 -> TEST3 with id 1
    let update_exchange =
        my_table.update_val_by_attr(ValByAttrQuery { val: 7777 }, "TEST2".to_string());
    let _ = task::block_on(update_exchange);

    let select_all_after_update = my_table.select_all();
    println!(
        "Select After Val Update by Attribute: {:?}",
        select_all_after_update.execute()
    );

    let test_delete = my_table.delete_by_attr("TEST3".to_string());
    let _ = task::block_on(test_delete);

    println!(
        "Select after deleted TEST3 {:?}",
        my_table.select_all().execute()
    );
}
