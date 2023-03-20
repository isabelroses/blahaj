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
                console.log(`Loaded command ${command.data.name}`);
            }
        }
        const clientId = '1087418361283092510';
        const guildId = '762413705383641158';
        const rest = new REST({ version: '10' }).setToken(process.env.token);
        try {
            console.log('Started refreshing application (/) commands.');
            await rest.put(Routes.applicationGuildCommands(clientId, guildId), {
                body: client.comamndArray
            });
            console.log('Successfully registered application commands.');
        } catch (error) {
            console.error(error);
        }
    }
}