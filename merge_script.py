import sqlite3
import os

def main():
    db_path = os.getenv("CHAT_DATABASE_PATH")

    if db_path is None:
        raise RuntimeError("CHAT_DATABASE_PATH is not specified")

    if not os.path.isdir(db_path):
        raise RuntimeError("CHAT_DATABASE_PATH is not directory")

        
    for file in os.listdir(db_path):
        filename = os.fsdecode(file)
        if filename.endswith(".db"):
            conn = sqlite3.connect(filename)
            cur = conn.cursor()
            version = cur.execute("""PRAGMA user_version""").fetchone()[0]
            if version < 1:
                cur.execute("ALTER TABLE forwarded_message ADD COLUMN sender_id INTEGER;")
                cur.execute("UPDATE forwarded_message SET sender_id = 0;")
                cur.execute("""PRAGMA user_version = 1""")
                conn.commit()
            
            conn.close()
        else:
            continue

if __name__ == "__main__":
    try:
        main()
    except RuntimeError as e:
        print(e)