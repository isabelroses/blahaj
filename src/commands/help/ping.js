const { SlashCommandBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('ping')
        .setDescription('Replies with Pong!'),
    async execute(interaction, client) {
        const msg = await interaction.deferReply({
            fetchReply: true
        });

        const newMessage = `API Latency is ${Math.round(client.ws.ping)}ms\nMessage Latency is ${msg.createdTimestamp - interaction.createdTimestamp}ms`;
        await interaction.editReply({
            content: newMessage
        });
    }
};