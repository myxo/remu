from telebot import types
import calendar as cal

# This function copied from https://github.com/unmonoqueteclea/calendar-telegram
def calendar(year, month, highlight_day = None):
    markup = types.InlineKeyboardMarkup()
    #First row - Month and Year
    row=[]
    row.append(types.InlineKeyboardButton(cal.month_name[month]+" "+str(year),callback_data="ignore"))
    markup.row(*row)

    my_calendar = cal.monthcalendar(year, month)
    for week in my_calendar:
        row=[]
        for day in week:
            if(day==0):
                row.append(types.InlineKeyboardButton(" ",callback_data="ignore"))
            else:
                day_str = str(day)
                if (highlight_day is not None and day == highlight_day):
                    day_str = '|' + day_str + '|'
                row.append(types.InlineKeyboardButton(day_str,callback_data="calendar-day-"+str(day)))
        markup.row(*row)
    #Last row - Buttons
    row=[]
    row.append(types.InlineKeyboardButton("<",callback_data="previous-month"))
    row.append(types.InlineKeyboardButton("today",callback_data="today"))
    row.append(types.InlineKeyboardButton("tomorrow",callback_data="tomorrow"))
    row.append(types.InlineKeyboardButton(">",callback_data="next-month"))
    markup.row(*row)
    return markup


def hour():
    markup = types.InlineKeyboardMarkup()
    row1 = [ types.InlineKeyboardButton(str(x*3), callback_data="time_hour:" + str(x*3)) for x in range(8)]
    row2 = [ types.InlineKeyboardButton(str(x*3 + 1), callback_data="time_hour:" + str(x*3 + 1)) for x in range(8)]
    row3 = [ types.InlineKeyboardButton(str(x*3 + 2), callback_data="time_hour:" + str(x*3 + 2)) for x in range(8)]
    markup.row(*row1)
    markup.row(*row2)
    markup.row(*row3)
            
    return markup


def minutes():
    markup = types.InlineKeyboardMarkup()
    row = []
    row.append(types.InlineKeyboardButton('00', callback_data='time_minute:00'))
    row.append(types.InlineKeyboardButton('15', callback_data='time_minute:15'))
    row.append(types.InlineKeyboardButton('30', callback_data='time_minute:30'))
    row.append(types.InlineKeyboardButton('45', callback_data='time_minute:45'))
    markup.row(*row)
    row = []
            
    return markup


def action():
    keyboard = types.InlineKeyboardMarkup()
    # callback_button_gr = types.InlineKeyboardButton(text="group", callback_data="group")
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
