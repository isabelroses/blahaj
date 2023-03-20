const { SlashCommandBuilder, ActionRowBuilder, ButtonBuilder, ButtonStyle } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('github')
        .setDescription('Responds with a button!'),
    async execute(interaction, client) {
        const button = new ButtonBuilder()
            .setLabel('Github')
            .setStyle(ButtonStyle.Link)
            .setURL('https://github.com/isabelroses');
        await interaction.reply({
            content: 'Click the button below to go to my Github!',
            components: [new ActionRowBuilder().addComponents(button)],
        });
    }
};