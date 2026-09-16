//! mineflayer Java bot. Username is always MincBot. Bedrock is out of scope.

const mineflayer = require("mineflayer");

function connectMincBot(opts = {}) {
  const host = opts.host || "127.0.0.1";
  const port = opts.port || 25565;
  const version = opts.version;
  const username = "MincBot";

  return new Promise((resolve, reject) => {
    let bot;
    try {
      bot = mineflayer.createBot({
        host,
        port,
        username,
        auth: "offline",
        version: version || undefined,
        hideErrors: true,
      });
    } catch (e) {
      reject(e);
      return;
    }

    const timer = setTimeout(() => {
      try {
        bot.end();
      } catch {
        /* ignore */
      }
      reject(new Error("MincBot spawn timeout"));
    }, opts.timeoutMs || 15000);

    const ok = () => {
      clearTimeout(timer);
      try {
        bot.chat("MincBot online");
      } catch {
        /* ignore */
      }
      resolve(bot);
    };

    bot.once("spawn", ok);
    bot.once("login", () => {
      // some flying-squid builds fire login without a full spawn
      setTimeout(() => {
        if (bot.entity) {
          ok();
        }
      }, 1500);
    });
    bot.once("error", (e) => {
      clearTimeout(timer);
      reject(e);
    });
    bot.once("kicked", (reason) => {
      clearTimeout(timer);
      reject(new Error(`kicked: ${reason}`));
    });
  });
}

function disconnect(bot) {
  if (!bot) {
    return;
  }
  try {
    bot.quit("done");
  } catch {
    try {
      bot.end();
    } catch {
      /* ignore */
    }
  }
}

module.exports = { connectMincBot, disconnect };
