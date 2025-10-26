use calendarize::calendarize_with_offset;
use chrono::NaiveDate;
use frankenstein::types::{InlineKeyboardButton, InlineKeyboardMarkup};

#[rustfmt::skip]
pub(crate) fn make_main_action_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![
        InlineKeyboardButton::builder().text("at").callback_data("at").build(),
        InlineKeyboardButton::builder().text("after").callback_data("after").build(),
    ]);

    keyboard.push(vec![
        InlineKeyboardButton::builder().text("5m").callback_data("5m").build(),
        InlineKeyboardButton::builder().text("30m").callback_data("30m").build(),
        InlineKeyboardButton::builder().text("1h").callback_data("1h").build(),
    ]);

    keyboard.push(vec![
        InlineKeyboardButton::builder().text("3h").callback_data("3h").build(),
        InlineKeyboardButton::builder().text("1d").callback_data("1d").build(),
        InlineKeyboardButton::builder().text("Ok").callback_data("Ok").build(),
    ]);

    InlineKeyboardMarkup {
        inline_keyboard: keyboard,
    }
}

pub fn make_calendar_keyboard(year: i32, month: u32) -> InlineKeyboardMarkup {
    let mut rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    let month_name = NaiveDate::from_ymd_opt(year, month, 1)
        .expect("valid date")
        .format("%B")
        .to_string(); // e.g. "October"
    let header = InlineKeyboardButton::builder()
        .text(format!("{} {}", month_name, year))
        .callback_data("ignore")
        .build();
    rows.push(vec![header]);

    let month_matrix =
        calendarize_with_offset(chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap(), 1);

    for week in month_matrix {
        let mut buttons: Vec<InlineKeyboardButton> = Vec::new();
        for day in week {
            let btn = match day {
                0 => InlineKeyboardButton::builder()
                    .text(" ")
                    .callback_data("ignore")
                    .build(),
                _ => InlineKeyboardButton::builder()
                    .text(day.to_string())
                    .callback_data(format!("calendar-day-{}", day))
                    .build(),
            };
            buttons.push(btn);
        }
        rows.push(buttons);
    }

    let footer = vec![
        InlineKeyboardButton::builder()
            .text("<")
            .callback_data("previous-month")
            .build(),
        InlineKeyboardButton::builder()
            .text("today")
            .callback_data("today")
            .build(),
        InlineKeyboardButton::builder()
            .text("tomorrow")
            .callback_data("tomorrow")
            .build(),
        InlineKeyboardButton::builder()
            .text(">")
            .callback_data("next-month")
            .build(),
    ];
    rows.push(footer);

    InlineKeyboardMarkup {
        inline_keyboard: rows,
    }
}

pub(crate) fn make_hour_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(
        (0..8)
            .map(|x| x * 3)
            .map(|x| {
                InlineKeyboardButton::builder()
                    .text(x.to_string())
                    .callback_data(format!("time_hour:{}", x))
                    .build()
            })
            .collect(),
    );
    keyboard.push(
        (0..8)
            .map(|x| x * 3 + 1)
            .map(|x| {
                InlineKeyboardButton::builder()
                    .text(x.to_string())
                    .callback_data(format!("time_hour:{}", x))
                    .build()
            })
            .collect(),
    );
    keyboard.push(
        (0..8)
            .map(|x| x * 3 + 2)
            .map(|x| {
                InlineKeyboardButton::builder()
                    .text(x.to_string())
                    .callback_data(format!("time_hour:{}", x))
                    .build()
            })
            .collect(),
    );

    InlineKeyboardMarkup {
        inline_keyboard: keyboard,
    }
}

#[rustfmt::skip]
pub(crate) fn make_minute_keyboard() -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

    keyboard.push(vec![
        InlineKeyboardButton::builder().text("00").callback_data("time_minute:00").build(),
        InlineKeyboardButton::builder().text("15").callback_data("time_minute:15").build(),
        InlineKeyboardButton::builder().text("30").callback_data("time_minute:30").build(),
        InlineKeyboardButton::builder().text("45").callback_data("time_minute:45").build(),
    ]);

    InlineKeyboardMarkup {
        inline_keyboard: keyboard,
    }
}
