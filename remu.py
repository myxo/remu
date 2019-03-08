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

@bot.message_handler(content_types=["text"])
def handle_text(message):
    input_text = message.text
    id = message.chat.id
    msg_id = message.message_id

    # Special case
    if input_text.find('/start') == 0:
        on_start_command(message)
        return

    print('handle_text function')
    
    command = engine.handle_text_message(message.chat.id, input_text)
    handle_backend_command(id, msg_id, command)
    return


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



@bot.callback_query_handler(func=lambda call: True)
def callback_inline(call):
    id = call.message.chat.id
    msg_id = call.message.message_id
    # This is a mandatory call. Mean we get query and processing it
    bot.answer_callback_query(call.id, text="")
    engine.log_debug("Processing keyboard callback. Call.data = %s."%(call.data))
    if call.message:
        command = engine.handle_keyboard_responce(id, call.data, call.message.text)
        handle_backend_command(id, msg_id, command)

def get_key(d):
    return list(d.keys())[0]

def handle_backend_command(uid, msg_id, command_str):
    print('id: %d, msg: %d, try to handle backend command: %s' % (uid, msg_id, command_str))
    command_vec = json.loads(command_str)
    print(command_vec)
    for command in command_vec:
        if get_key(command) == 'send':
            bot.send_message(uid, command['send']['text'], parse_mode='Markdown')
        
        elif get_key(command) == 'calendar':
            handle_calendar_call(uid, msg_id, command['calendar'])

        elif get_key(command) == 'delete_message':
            bot.delete_message(chat_id=uid, message_id=msg_id)
        
        elif get_key(command) == 'delete_keyboard':
            bot.edit_message_reply_markup(chat_id=uid, message_id=msg_id)

        elif get_key(command) == 'keyboard':
            if command['keyboard']['action_type'] == 'hour':
                handle_hour_keyboard(uid, msg_id)
            
            if command['keyboard']['action_type'] == 'minute':
                handle_minutes_keyboard(uid, msg_id, command['keyboard']['text'])

            if command['keyboard']['action_type'] == 'main':
                keyboard = keyboards.action()
                bot.send_message(uid, command['keyboard']['text'], reply_markup=keyboard)
            

# ------------- helper function


def handle_calendar_call(chat_id, msg_id, command):
    month = command['month']
    year = command['year']
    markup = keyboards.calendar(year, month)
    if command['edit_msg'] == True:
        bot.edit_message_text("Please, choose a date", chat_id, msg_id, reply_markup=markup)
    else: 
        bot.send_message(chat_id, "Please, choose a date", reply_markup=markup)



def handle_hour_keyboard(chat_id, prev_msg_id):
    bot.delete_message(chat_id, prev_msg_id)

    markup = keyboards.hour()
    bot.send_message(chat_id, "Please, choose hour", reply_markup=markup)

def handle_minutes_keyboard(chat_id, prev_msg_id, text):
    bot.delete_message(chat_id, prev_msg_id)

    markup = keyboards.minutes()
    keyboard_message = bot.send_message(chat_id, text, reply_markup=markup)


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
    # for chat_id in user_chat_id_list:
        # fsm[chat_id] = FSMData()

    engine.run()


    while True:
        try:
            bot.polling()
        except:
            engine.log_error("I am down =(")

        if one_poll:
            break
        time.sleep(2)

    engine.stop()
