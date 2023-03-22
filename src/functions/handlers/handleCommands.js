const { REST } = require('@discordjs/rest');
const { Routes } = require('discord-api-types/v9');
const fs = require('fs');

module.exports = (client) => {
    client.handleCommands = async () => {
        const commandFolders = fs.readdirSync('./src/commands');
        for (const folder of commandFolders) {
            const commandFiles = fs.readdirSync(`./src/commands/${folder}`).filter(file => file.endsWith('.js'));
            const { commands, comamndArray } = client;
            for (const file of commandFiles) {
                const command = require(`../../commands/${folder}/${file}`);
                commands.set(command.data.name, command);
                comamndArray.push(command.data.toJSON());
            }
        }
        const clientId = process.env.clientId;
        const guild_ids = client.guilds.cache.map(guild => guild.id);
        const rest = new REST({ version: '10' }).setToken(process.env.token);
        try {
            console.log('Started refreshing application (/) commands.');
            for (const guild_id of guild_ids) {
                await rest.put(
                    Routes.applicationGuildCommands(clientId, guild_id),
                    { body: comamndArray },
                ).then(() => console.log(`Successfully reloaded application (/) commands for guild ${guild_id}.`)).catch(console.error);
            }
        } catch (error) {
            console.error(error);
        }
    }
}