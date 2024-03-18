require('dotenv').config();
const { Client, Collection, GatewayIntentBits } = require('discord.js');
const fs = require('fs');
const path = require('path');

const client = new Client({ intents: GatewayIntentBits.Guilds });
client.commands = new Collection();
client.commandArray = [];
client.buttons = new Collection();

const directoryPath = path.join(__dirname, 'functions');
const functionFolders = fs.readdirSync(directoryPath);

for (const folder of functionFolders) {
    const functionFiles = fs.readdirSync(path.join(directoryPath, folder)).filter(file => file.endsWith('.js'));
    for (const file of functionFiles) require(path.join(directoryPath, folder, file))(client);
}

client.handleComponents();
client.handleEvents();
client.login(process.env.DISCORD_TOKEN);
