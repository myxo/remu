import argparse
import datetime
import subprocess
import threading
import time
import os
from enum import Enum

import telebot

import libremu_backend as engine
import text_data as text
import keyboards


f = open('token.id', 'r')
token = f.read()
f.close()
f = open('yandex_api.id', 'r')
yandex_api_token = f.read()
f.close()
bot = telebot.TeleBot(token)
fsm = {}
current_shown_dates={} # TODO: get rid off

class BotState(Enum):
    WAIT                = 0
    REP_DELETE_CHOOSE   = 1
    AT_CALENDAR         = 2
    AT_TIME_TEXT        = 3
    AFTER_INPUT         = 4
    GROUPE_CHOOSE       = 5
    GROUP_ADD_ITEM      = 6
    GROUP_DEL_ITEM      = 7
    GROUP_DEL           = 8

class FSMData:
    state = BotState.WAIT
    data = {}

    def reset(self):
        self.state = BotState.WAIT
        self.data = {}


@bot.message_handler(content_types=["text"])
def handle_text(message):
    input_text = message.text
    id = message.chat.id

    engine.log_debug("Processing input text message: %s. Bot state = %s"%(input_text, str(fsm[id].state)))

    if fsm[id].state == BotState.WAIT: 
        on_wait_status(message)
    
    elif fsm[id].state == BotState.REP_DELETE_CHOOSE:
        on_rep_delete_choose_status(message)

    elif fsm[id].state == BotState.AT_CALENDAR:
        on_at_calendar_status(message)

    elif fsm[id].state == BotState.AT_TIME_TEXT:
        on_at_time_text_status(message)

    elif fsm[id].state == BotState.AFTER_INPUT:
        on_after_input_status(message)

    elif fsm[id].state == BotState.GROUP_ADD_ITEM:
        on_group_add_item_status(message)

    elif fsm[id].state == BotState.GROUP_DEL_ITEM:
        on_group_del_item_status(message)
    
    else:
        engine.log_error("Unknown bot state: uid = " + str(id) + " state = " + str(fsm[id].state))
        fsm[id].reset()
        handle_text(message)


def on_wait_status(message):
    input_text = message.text
    id = message.chat.id
    
    if input_text.find('/start') == 0:
        on_start_command(message)

    elif input_text.find('/help') == 0:
        on_help_command(message)
    
    elif input_text.find('/delete_rep') == 0:
        on_delete_rep_command(message)
    
    elif input_text.find('/at') == 0:
        on_at_command(message)

    elif input_text.find('/group') == 0:
        on_group_command(message)

    elif input_text.find('/add_group') == 0:
        on_add_group_command(message)

    elif input_text.find('/del_group') == 0:
        on_del_group_command(message)

    elif input_text.find('/del_group_item') == 0:
        on_del_group_item_command(message)

    elif input_text.find('/list') == 0:
        on_list_command(message)

    else:
        (text, error) = engine.handle_text_message(message.chat.id, input_text)
        if error == 0:
            bot.send_message(message.chat.id, text)
        else:
            keyboard = keyboards.action()
            bot.send_message(id, input_text, reply_markup=keyboard)


def on_rep_delete_choose_status(message):
    delete_rep_event(message)


def on_at_calendar_status(message):
    id = message.chat.id
    message_id = fsm[id].data['message_id']
    bot.delete_message(chat_id=id, message_id=message_id)
    fsm[id].reset()
    handle_text(message)
    

def on_at_time_text_status(message):
    id = message.chat.id
    input_text = message.text
    if fsm[id].data['text']:
        input_text += ' ' + fsm[id].data['text']
    command = fsm[id].data['date_spec'] + ' at ' + input_text
    bot.send_message(id, 'Resulting command:\n' + command)
    (text, _) = engine.handle_text_message(id, command)
    bot.send_message(id, text)
    fsm[id].reset()


def on_after_input_status(message):
    command = message.text + ' ' + fsm[id].data['text']
    bot.send_message(id, 'Resulting command:\n' + command)
    (text, _) = engine.handle_text_message(id, command)
    bot.send_message(id, text)
    fsm[id].reset()


def on_group_add_item_status(message):
    # should be cathced in keyboard handle
    pass 

def on_group_del_item_status(message):
    id = message.chat.id
    event_id_str = message.text
    if not event_id_str.isdigit():
        msg = bot.reply_to(message, 'You should write number')
        return

    event_id = int(event_id_str)-1
    id_list = fsm[id].data['id_list']
    if id_list and event_id >= 0 and event_id < len(id_list):
        del_id = id_list[event_id]
        engine.delete_group_item(del_id)
        fsm[id].reset()
        bot.send_message(id, "Done.")
    else:
        fsm[id].reset()
        bot.send_message(id, "Number is out of limit. Operation abort.")
    pass 

# ------------------- command handlers


def on_start_command(message):
    engine.add_user(message.from_user.id, message.from_user.username, message.chat.id, -3)
    bot.send_message(message.chat.id, 'Hello! ^_^\nType /help')

def on_help_command(message):
    bot.send_message(message.chat.id, text.main_help_message_ru, parse_mode='Markdown')

def on_delete_rep_command(message):
    global fsm
    uid = message.chat.id
    fsm[uid].state = BotState.WAIT
    rep_event_list = engine.get_rep_events(uid)

    if not rep_event_list:
        bot.send_message(message.chat.id, 'No current rep event')
        return

    [text_list, rep_id_list] = list(zip(*rep_event_list))
    fsm[uid].state = BotState.REP_DELETE_CHOOSE
    fsm[uid].data['rep_id_list'] = rep_id_list

    header = "Here is yout rep events list. Choose witch to delete:\n"
    list_str = '\n'.join([ str(i+1) + ") " + key for i, key in enumerate(text_list)])
    bot.send_message(uid, header + list_str)

def on_at_command(message):
    handle_calendar_call(message.chat.id)

def on_group_command(message):
    choose_group_message(message.from_user.id)

def on_add_group_command(message):
    uid = message.chat.id
    offset = len('/add_group ')
    if offset >= len(message.text):
        bot.send_message(uid, 'You should write group name')
    group_name = message.text[offset:]
    engine.add_user_group(uid, group_name)
    bot.send_message(uid, 'Done.')


def on_del_group_command(message):
    choose_group_message(message.chat.id, next_state=BotState.GROUP_DEL)

def on_list_command(message):
    text_list = engine.get_active_events(message.from_user.id)
    if not text_list:
        bot.send_message(message.chat.id, 'No current active event')
    list_str = '\n'.join([ str(i+1) + ") " + key for i, key in enumerate(text_list)])
    bot.send_message(message.chat.id, list_str, parse_mode='Markdown')


def on_del_group_item_command(message):
    choose_group_message(message.chat.id, next_state=BotState.GROUP_DEL_ITEM)



# --------------- Keyboard callback handlers 


@bot.callback_query_handler(func=lambda call: call.data == 'next-month' or call.data == 'previous-month')
def change_month(call):
    next_month = call.data == 'next-month'
    chat_id = call.message.chat.id
    saved_date = current_shown_dates.get(chat_id)
    if saved_date is None:
        engine.log_error("Called calendar change_month handler, but there no saved_date by " + str(chat_id) + " chat_id")
        return

    year, month = saved_date
    if next_month:
        month += 1
        if month > 12:
            month, year = (1, year+1)
    else:
        month -= 1
        if month < 1:
            month, year = (12, year-1)

    current_shown_dates[chat_id] = (year, month)
    markup = keyboards.calendar(year, month)
    bot.edit_message_text("Please, choose a date", call.from_user.id, call.message.message_id, reply_markup=markup)
    bot.answer_callback_query(call.id, text="")


@bot.callback_query_handler(func=lambda call: call.data[0:13] == 'calendar-day-')
def get_day(call):
    chat_id = call.message.chat.id
    saved_date = current_shown_dates.get(chat_id)
    if(saved_date is None):
        engine.log_error("Called calendar get_day handler, but there no saved_date by " + str(chat_id) + " chat_id")
        return 

    day = call.data[13:]
    date = datetime.datetime(int(saved_date[0]), int(saved_date[1]), int(day), 0, 0, 0)
    fsm[chat_id].state = BotState.AT_TIME_TEXT
    fsm[chat_id].data['date_spec'] = day + '-' + str(saved_date[1]) + '-' + str(saved_date[0])
    # delete keyboard
    bot.edit_message_text(chat_id=call.message.chat.id, message_id=call.message.message_id, text=call.message.text)
    if fsm[chat_id].data['text']:
        bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time of event.')
    else:
        bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time and text of event.')
    bot.answer_callback_query(call.id, text="")


@bot.callback_query_handler(func=lambda call: call.data[0:3] == 'grp')
def on_select_group(call):
    uid = call.message.chat.id
    gid = int(call.data[3:])
    engine.log_debug("Processing keyboard callback. Call.data = %s. Bot state = %s"%(call.data, str(fsm[uid].state)))
    if fsm[uid].state == BotState.GROUP_ADD_ITEM:
        text = fsm[uid].data['text']
        engine.add_group_item(gid, text)
        bot.send_message(uid, 'Done')
    elif fsm[uid].state == BotState.GROUP_DEL:
        engine.delete_user_group(gid)
        bot.send_message(uid, 'Done')
    else:
        items = engine.get_group_items(gid)
        if items:
            [text_list, id_list] = list(zip(*items)) # TODO: add group name
            text = '\n'.join([ str(i+1) + ') ' + item for i, item in enumerate(text_list) ])
            fsm[uid].data['id_list'] = id_list
            bot.send_message(uid, text)
        else:
            bot.send_message(uid, 'No items in group')

    bot.delete_message(uid, fsm[uid].data['message_id'])
    if fsm[uid].state == BotState.GROUP_DEL_ITEM:
        bot.send_message(uid, 'Choose element to delete')
    else:
        fsm[uid].reset()


@bot.callback_query_handler(func=lambda call: True)
def callback_inline(call):
    id = call.message.chat.id
    engine.log_debug("Processing keyboard callback. Call.data = %s. Bot state = %s"%(call.data, str(fsm[id].state)))
    if call.message:
        if call.data == 'at':
            handle_calendar_call(id, call.message.text)
        elif call.data == 'after':
            fsm[id].state = BotState.AFTER_INPUT
            fsm[id].data['text'] = call.message.text
            bot.send_message(id, 'Ok, now write time duration.')
        elif call.data == 'group':
            fsm[id].state = BotState.GROUP_ADD_ITEM
            fsm[id].data['text'] = call.message.text
            choose_group_message(id, next_state=BotState.GROUP_ADD_ITEM, add_if_not_exist=False)
        elif call.data != "Ok":
            call.message.text = call.data + " " + call.message.text
            handle_text(call.message)
        # delete keys
        bot.edit_message_text(chat_id=call.message.chat.id, message_id=call.message.message_id, text=call.message.text)



# ------------- helper function


def delete_rep_event(message):
    event_id_str = message.text
    if not event_id_str.isdigit():
        msg = bot.reply_to(message, 'You should write number')
        return

    event_id = int(event_id_str)-1
    id_list = fsm[message.chat.id].data['rep_id_list']
    if id_list and event_id >= 0 and event_id < len(id_list):
        del_id = id_list[event_id]
        engine.del_rep_event(del_id)
        fsm[message.chat.id].reset()
        bot.send_message(message.chat.id, "Done.")
    else:
        fsm[message.chat.id].reset()
        bot.send_message(message.chat.id, "Number is out of limit. Operation abort.")


def handle_calendar_call(chat_id, text=None):
    now = datetime.datetime.now()
    date = (now.year,now.month)
    current_shown_dates[chat_id] = date
    markup = keyboards.calendar(now.year,now.month)
    fsm[chat_id].state = BotState.AT_CALENDAR
    fsm[chat_id].data['text'] = text
    keyboard_message = bot.send_message(chat_id, "Please, choose a date", reply_markup=markup)
    fsm[chat_id].data['message_id'] = keyboard_message.message_id


def choose_group_message(id, next_state=None, add_if_not_exist=False):
    groups = engine.get_user_groups(id)
    if not groups:
        bot.send_message(id, 'No groups.') # TODO: добавить группу
        return
    [text_list, id_list] = list(zip(*groups))
    if next_state:
        fsm[id].state = next_state
    keyboard = keyboards.groups(text_list, id_list)
    keyboard_message = bot.send_message(id, 'Choose group.', reply_markup=keyboard)
    fsm[id].data['message_id'] = keyboard_message.message_id


@bot.message_handler(content_types=['voice'])
def voice_processing(message):
    engine.log_debug('Start to processing voice. File id = ' + message.voice.file_id)
    file_info = bot.get_file(message.voice.file_id)
    file = bot.download_file(file_info.file_path)
    if not os.path.exists('voice'):
        os.makedirs('voice')
    filename_ogg = message.voice.file_id + '.ogg'
    filename_wav = message.voice.file_id + '.wav'
    with open('voice/' + filename_ogg, 'wb') as f:
        f.write(file)

    command = [
        'opusdec',
        '--rate', '16000',
        '--force-wav',
        '--quiet',
        'voice/' + filename_ogg,
        'voice/' + filename_wav
    ]
    proc = subprocess.Popen(command,
                           stdout=subprocess.PIPE, 
                           stderr=subprocess.PIPE)
    err = proc.stderr.read()
    if err: engine.log_error(err)

    command = [
        'asrclient-cli.py',
        '--key=' + yandex_api_token,
        '--silent',
        'voice/' + filename_wav
    ]
    proc = subprocess.Popen(' '.join(command), shell=True,
                           stdout=subprocess.PIPE, 
                           stderr=subprocess.PIPE)
    result = []
    for line in proc.stdout:
        result.append(line)
    
    err = proc.stderr.read()
    if err: engine.log_error(err)

    engine.log_info('Speech rec result: ' + str(result))

    if len(result) <= 1:
        bot.send_message(message.chat.id, "Can't recognize =(")
    else:
        keyboard = keyboards.action()
        bot.send_message(message.chat.id, result[:-1], reply_markup=keyboard)



def on_engine_event(text, chat_id):
    keyboard = keyboards.action()
    bot.send_message(chat_id, text, reply_markup=keyboard)



if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument("-v", 
                        "--verbose", 
                        help="show log lines in stdout",
                        action="store_true")
    parser.add_argument("--one-poll", 
                        help="do not try to polling againg after error",
                        action="store_true")
    args = parser.parse_args()
    verbose = True if args.verbose else False
    one_poll = True if args.one_poll else False

    engine.initialize(verbose)
    engine.register_callback(on_engine_event)
    user_chat_id_list = engine.get_user_chat_id_all()
    for chat_id in user_chat_id_list:
        fsm[chat_id] = FSMData()

    engine.run()

    while True:
        try:
            bot.polling()
        except:
            engine.log_error("I am down =(")

        if one_poll:
            break

    engine.stop()
