import argparse
import datetime
import subprocess
import threading
import time
import os
import json
import pprint
from enum import Enum

import telebot

import libremu_backend as engine
import text_data as text
import keyboards

import config

base_error_message = 'There are some problem with request. Forward messages to @nikolay_klimov if you think this is a bug.'

bot = telebot.TeleBot(config.tg_token, num_threads=4)
fsm = {} # Finite State Machine
current_shown_dates={} # TODO: get rid off

class BotState(Enum):
    WAIT                = 0
    REP_DELETE_CHOOSE   = 1
    AT_CALENDAR         = 2
    AT_TIME             = 10
    AT_TIME_TEXT        = 3
    AFTER_INPUT         = 4


class FSMData:
    state = BotState.WAIT
    data = {}

    def reset(self):
        self.state = BotState.WAIT
        self.data = {}


@bot.message_handler(content_types=["text"])
def handle_text(message):
    input_text = message.text
    # import pdb; pdb.set_trace()
    id = message.chat.id
    msg_id = message.message_id
    # if input_text.find('/at') == 0:
        # on_at_command(message)
        # return

    print('handle_text function')
    
    command = engine.handle_text_message(message.chat.id, input_text)
    # FIXME: should not be error, but command
    # if error == 0 and command == 'send_message':
    #     bot.send_message(message.chat.id, text)
    # else:
    #     keyboard = keyboards.action()
    #     bot.send_message(id, input_text, reply_markup=keyboard)
    handle_backend_command(id, msg_id, command)
    return

    # Special case
    if input_text.find('/start') == 0:
        on_start_command(message)
        fsm[id] = FSMData()
        return

    engine.log_debug("Processing input text message: %s. Bot state = %s"%(input_text, str(fsm[id].state)))

    if fsm[id].state == BotState.WAIT:
        # on_wait_status(message)
        pass
    
    # elif fsm[id].state == BotState.REP_DELETE_CHOOSE:
    #     on_rep_delete_choose_status(message)

    elif fsm[id].state == BotState.AT_CALENDAR:
        on_at_calendar_status(message)

    elif fsm[id].state == BotState.AT_TIME_TEXT:
        on_at_time_text_status(message)

    elif fsm[id].state == BotState.AFTER_INPUT:
        on_after_input_status(message)

    elif fsm[id].state == BotState.AT_TIME:
        on_at_time_status(message)
    
    else:
        engine.log_error("Unknown bot state: uid = " + str(id) + " state = " + str(fsm[id].state))
        fsm[id].reset()
        handle_text(message)


# def on_wait_status(message):
#     input_text = message.text
#     id = message.chat.id

    # if input_text.find('/help') == 0:
    #     on_help_command(message)
    
#     elif input_text.find('/delete_rep') == 0:
#         on_delete_rep_command(message)


    # elif input_text.find('/list') == 0:
    #     on_list_command(message)

    # else:
    #     (text, error) = engine.handle_text_message(message.chat.id, input_text)
    #     if error == 0:
    #         bot.send_message(message.chat.id, text)
    #     else:
    #         keyboard = keyboards.action()
    #         bot.send_message(id, input_text, reply_markup=keyboard)


# def on_rep_delete_choose_status(message):
#     delete_rep_event(message)


def on_at_calendar_status(message):
    id = message.chat.id
    message_id = fsm[id].data['message_id']
    bot.delete_message(chat_id=id, message_id=message_id)
    fsm[id].reset()
    handle_text(message)


def on_at_time_status(message):
    # id = message.chat.id
    # keyboard = keyboards.time()
    # bot.send_message(id, )
    pass
    

def on_at_time_text_status(message):
    print('on_at_time_text_status function')
    id = message.chat.id
    input_text = message.text
    if fsm[id].data['text']:
        input_text += ' ' + fsm[id].data['text']
    command = fsm[id].data['date_spec'] + ' at ' + input_text
    bot.send_message(id, 'Resulting command:\n' + command)
    (text, err) = engine.handle_text_message(id, command)
    if err:
        bot.send_message(id, 'Wrong resulting command. Try again')
    else:
        bot.send_message(id, text)
    fsm[id].reset()


def on_after_input_status(message):
    print('on_after_input_status function')
    id = message.chat.id
    command = message.text + ' ' + fsm[id].data['text']
    bot.send_message(id, 'Resulting command:\n' + command)
    (text, _) = engine.handle_text_message(id, command)
    bot.send_message(id, text)
    fsm[id].reset()


# ------------------- command handlers


def on_start_command(message):
    username = ''
    first_name = ''
    last_name = ''
    if message.from_user.username: username = message.from_user.username
    if message.from_user.first_name: first_name = message.from_user.first_name
    if message.from_user.last_name: last_name = message.from_user.last_name
    engine.add_user(message.from_user.id, 
                    username, 
                    message.chat.id, 
                    first_name, 
                    last_name, 
                    -3)
    bot.send_message(message.chat.id, 'Hello! ^_^\nType /help')


# --------------- Keyboard callback handlers 


@bot.callback_query_handler(func=lambda call: call.data[0:13] == 'calendar-day-')
def get_day(call):
    engine.log_debug("Processing button callback: %s"%(call.data))
    chat_id = call.message.chat.id
    saved_date = current_shown_dates.get(chat_id)
    # import pdb; pdb.set_trace()
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
    handle_hour_keyboard(chat_id)


@bot.callback_query_handler(func=lambda call: call.data == 'today' or call.data == 'tomorrow')
def on_calendar_today(call):
    engine.log_debug("Processing button callback: %s"%(call.data))
    chat_id = call.message.chat.id

    date = datetime.datetime.now()
    if call.data == 'tomorrow':
        date += datetime.timedelta(days=1)
    # date = datetime.datetime(int(saved_date[0]), int(saved_date[1]), int(day), 0, 0, 0)
    fsm[chat_id].state = BotState.AT_TIME_TEXT
    fsm[chat_id].data['date_spec'] = str(date.day) + '-' + str(date.month) + '-' + str(date.year)
    # delete keyboard
    bot.edit_message_text(chat_id=call.message.chat.id, message_id=call.message.message_id, text=call.message.text)
    if fsm[chat_id].data['text']:
        bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time of event.')
    else:
        bot.send_message(chat_id, 'Ok, ' + date.strftime(r'%b %d') + '. Now write the time and text of event.')
    bot.answer_callback_query(call.id, text="")
    handle_hour_keyboard(chat_id)



@bot.callback_query_handler(func=lambda call: call.data[:10] == 'time_hour:')
def on_time_hour(call):
    engine.log_debug("Processing button callback: %s"%(call.data))
    chat_id = call.message.chat.id
    hour = int(call.data[10:])
    fsm[chat_id].data['hour'] = hour
    handle_minutes_keyboard(chat_id)

@bot.callback_query_handler(func=lambda call: call.data[:12] == 'time_minute:')
def on_time_minutes(call):
    print('on_time_minutes function')
    engine.log_debug("Processing button callback: %s"%(call.data))
    chat_id = call.message.chat.id
    minute = int(call.data[12:])
    message_id = fsm[chat_id].data['message_id']
    bot.delete_message(chat_id, message_id)

    hour = fsm[chat_id].data['hour'] 
    text = fsm[chat_id].data['text'] 
    date_spec = fsm[chat_id].data['date_spec'] 
    result_command = date_spec + ' at ' + str(hour) + '.' + str(minute) + ' ' + text
    (reply, err) = engine.handle_text_message(chat_id, result_command)
    if err:
        bot.send_message(chat_id, base_error_message)
    else:
        bot.send_message(chat_id, reply)
    fsm[chat_id].reset()


@bot.callback_query_handler(func=lambda call: True)
def callback_inline(call):
    id = call.message.chat.id
    msg_id = call.message.message_id
    # Telegram clients will display a progress bar until this call 
    bot.answer_callback_query(call.id, text="")
    engine.log_debug("Processing keyboard callback. Call.data = %s. Bot state = %s"%(call.data, str(fsm[id].state)))
    if call.message:
        if call.data == 'at' or call.data == 'next-month' or call.data == 'previous-month' or call.data == 'after':
            command = engine.handle_keyboard_responce(id, call.data)
            handle_backend_command(id, msg_id, command)
            return
        elif call.data == 'Ok':
            bot.edit_message_reply_markup(chat_id=call.message.chat.id, message_id=call.message.message_id)
            return
        elif call.data == 'ignore':
            fsm[id].reset() 
        elif call.data != "Ok":
            call.message.text = call.data + " " + call.message.text
            handle_text(call.message)
        bot.delete_message(chat_id=call.message.chat.id, message_id=call.message.message_id)

def get_key(d):
    return list(d.keys())[0]

def handle_backend_command(uid, msg_id, command_str):
    print('id: %d, msg: %d, try to handle backend command: %s' % (uid, msg_id, command_str))
    command_vec = json.loads(command_str)
    print(command_vec)
    for command in command_vec:
        if get_key(command) == 'send':
            bot.send_message(uid, command['send']['text'])
        
        elif get_key(command) == 'calendar':
            handle_calendar_call(uid, msg_id, command['calendar'])

        elif get_key(command) == 'keyboard':
            if command['keyboard']['action_type'] == 'hour':
                print('hour keyboard')
            
            if command['keyboard']['action_type'] == 'minute':
                print('minute keyboard')

# ------------- helper function


def handle_calendar_call(chat_id, msg_id, command):
    month = command['month']
    year = command['year']
    markup = keyboards.calendar(year, month)
    if command['edit_msg'] == True:
        bot.edit_message_text("Please, choose a date", chat_id, msg_id, reply_markup=markup)
    else: 
        keyboard_message = bot.send_message(chat_id, "Please, choose a date", reply_markup=markup)
        fsm[chat_id].data['message_id'] = keyboard_message.message_id



def handle_hour_keyboard(chat_id):
    if fsm[chat_id].data['message_id']:
        message_id = fsm[chat_id].data['message_id']
        bot.delete_message(chat_id, message_id)

    markup = keyboards.hour()
    fsm[chat_id].state = BotState.AT_TIME
    keyboard_message = bot.send_message(chat_id, "Please, choose hour", reply_markup=markup)
    fsm[chat_id].data['message_id'] = keyboard_message.message_id

def handle_minutes_keyboard(chat_id):
    if fsm[chat_id].data['message_id']:
        message_id = fsm[chat_id].data['message_id']
        bot.delete_message(chat_id, message_id)

    markup = keyboards.minutes()
    fsm[chat_id].state = BotState.AT_TIME
    hour = fsm[chat_id].data['hour']
    text = 'Ok, %d. Now choose minute.'%hour
    keyboard_message = bot.send_message(chat_id, text, reply_markup=markup)
    fsm[chat_id].data['message_id'] = keyboard_message.message_id


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
        '--key=' + config.yandex_api_token,
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
            for key in fsm:
                fsm[key].reset()

        if one_poll:
            break
        time.sleep(2)

    engine.stop()
