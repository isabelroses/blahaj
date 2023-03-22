const { SlashCommandBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('invite')
        .setDescription('Replies with the bot\'s invite link!'),
    async execute(interaction) {
        await interaction.reply({ content: 'https://discord.com/api/oauth2/authorize?client_id=1087418361283092510&permissions=8&scope=bot%20applications.commands' });
    },
};