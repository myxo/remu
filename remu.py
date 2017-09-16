import telebot
import logging
import time
import threading
# from datetime import date, time, datetime, timedelta

import libtelegram_rust_backend as engine

logging.basicConfig(filename='log.txt', format='[%(asctime)s] [%(levelname)s]  %(message)s', level=logging.INFO)

f = open('token.id', 'r')
token = f.read()
bot = telebot.TeleBot(token)
chat_id = 0

# @bot.message_handler(content_types=['document', 'audio'])
# def handle_docs_audio(message):
#     file_info = bot.get_file(message.document.file_id)
#     file = bot.download_file(file_info.file_path)
#     with open(message.document.file_name, 'wb') as f:
#         f.write(file)


@bot.message_handler(content_types=["text"])
def repeat_all_messages(message):
    global chat_id
    if chat_id != message.chat.id:
        logging.info('chat_id changed!')
    chat_id = message.chat.id
    handle_user_message(message.text)


@bot.callback_query_handler(func=lambda call: True)
def callback_inline(call):
    if call.message:
        handle_user_message(call.data)


def handle_user_message(message_text):
    text = engine.handle_text_message(message_text)
    bot.send_message(chat_id, text)


# temporal solution
def run_check_loop():
    while True:
        text = engine.check_for_message()
        if text != "":
            send_message(text)
        time.sleep(0.5)

def send_message(message_text):
    keyboard = telebot.types.InlineKeyboardMarkup()
    callback_data = '1h ' + message_text
    callback_button = telebot.types.InlineKeyboardButton(text="1h", callback_data=callback_data)
    keyboard.add(callback_button)
    bot.send_message(chat_id, message_text, reply_markup=keyboard)

if __name__ == '__main__':
    # engine.register_action_callback( lambda text: bot.send_message(chat_id, text))
    engine.initialize()
    # engine.run()
    threading.Thread(target=run_check_loop).start()
    bot.polling(none_stop=True)

    engine.stop()
