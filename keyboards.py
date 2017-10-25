from telebot import types
import calendar

# This function copied from https://github.com/unmonoqueteclea/calendar-telegram
def calendar(year,month):
    markup = types.InlineKeyboardMarkup()
    #First row - Month and Year
    row=[]
    row.append(types.InlineKeyboardButton(calendar.month_name[month]+" "+str(year),callback_data="ignore"))
    markup.row(*row)

    my_calendar = calendar.monthcalendar(year, month)
    for week in my_calendar:
        row=[]
        for day in week:
            if(day==0):
                row.append(types.InlineKeyboardButton(" ",callback_data="ignore"))
            else:
                row.append(types.InlineKeyboardButton(str(day),callback_data="calendar-day-"+str(day)))
        markup.row(*row)
    #Last row - Buttons
    row=[]
    row.append(types.InlineKeyboardButton("<",callback_data="previous-month"))
    row.append(types.InlineKeyboardButton(" ",callback_data="ignore"))
    row.append(types.InlineKeyboardButton(">",callback_data="next-month"))
    markup.row(*row)
    return markup


def action():
    keyboard = types.InlineKeyboardMarkup()
    callback_button_at = types.InlineKeyboardButton(text="at", callback_data="at")
    callback_button_after = types.InlineKeyboardButton(text="after", callback_data="after")
    keyboard.add(callback_button_at, callback_button_after)

    callback_button_5m = types.InlineKeyboardButton(text="5m", callback_data="5m")
    callback_button_30m = types.InlineKeyboardButton(text="30m", callback_data="30m")
    callback_button_1h = types.InlineKeyboardButton(text="1h", callback_data="1h")
    keyboard.add(callback_button_5m, callback_button_30m, callback_button_1h)
    
    callback_button_3h = types.InlineKeyboardButton(text="3h", callback_data="3h")
    callback_button_1d = types.InlineKeyboardButton(text="1d", callback_data="1d")
    callback_button_ok = types.InlineKeyboardButton(text="Ok", callback_data="Ok")
    keyboard.add(callback_button_3h, callback_button_1d, callback_button_ok)
    return keyboard


def groups(text_list, id_list):
    keyboard = types.InlineKeyboardMarkup()
    for i, text in enumerate(text_list):
        callback_button = types.InlineKeyboardButton(text=text, callback_data='grp'+str(id_list[i]))
        keyboard.add(callback_button)
    return keyboard