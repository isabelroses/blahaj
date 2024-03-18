const { REST } = require('@discordjs/rest');
const { Routes } = require('discord-api-types/v9');
const fs = require('fs');

module.exports = (client) => {
    client.handleCommands = async () => {
        const commandFolders = fs.readdirSync('./src/commands');
        for (const folder of commandFolders) {
            const commandFiles = fs.readdirSync(`./src/commands/${folder}`).filter(file => file.endsWith('.js'));
            const { commands, commandArray } = client;
            for (const file of commandFiles) {
                const command = require(`../../commands/${folder}/${file}`);
                commands.set(command.data.name, command);
                commandArray.push(command.data.toJSON());
            }
        }
        const guild_ids = client.guilds.cache.map(guild => guild.id);
        const rest = new REST({ version: '10' }).setToken(process.env.DISCORD_TOKEN);
        try {
            console.log('Started refreshing application (/) commands.');
            for (const guildId of guild_ids) {
                await rest.put(Routes.applicationGuildCommands(process.env.CLIENT_ID, guildId),
                    { body: client.commandArray }
                ).then(() => console.log('Successfully updated commands for guild ' + guildId)).catch(console.error);
            }
        } catch (error) {
            console.error(error);
        }
    }
}
