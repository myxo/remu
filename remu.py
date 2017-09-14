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
    text = engine.handle_text_message(message.text)
    bot.send_message(message.chat.id, text)
    global chat_id
    chat_id = message.chat.id


# temporal solution
def run_check_loop():
    while True:
        text = engine.check_for_message()
        if text != "":
            bot.send_message(chat_id, text)
        time.sleep(0.5)

if __name__ == '__main__':
    # engine.register_action_callback( lambda text: bot.send_message(chat_id, text))
    engine.initialize()
    # engine.run()
    threading.Thread(target=run_check_loop).start()
    bot.polling(none_stop=True)

    engine.stop()
