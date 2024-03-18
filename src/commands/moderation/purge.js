const { SlashCommandBuilder } = require('@discordjs/builders');
const { PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('purge')
        .setDescription('Deletes messages.')
        .setDefaultMemberPermissions(PermissionsBitField.Flags.ManageMessages)
        .addIntegerOption(option =>
            option.setName('amount')
                .setMinValue(1)
                .setMaxValue(100)
                .setDescription('The amount of messages to delete.')
                .setRequired(true)),
    async execute(interaction) {
        if (!interaction.member.permissions.has(PermissionsBitField.Flags.ManageMessages)) return await interaction.reply({ content: 'You do not have permission to purge messages', ephemeral: true })

        let amount = interaction.options.getInteger('amount');

        await interaction.channel.bulkDelete(amount).catch(err => {
            console.error(err);
            interaction.reply({ content: 'There was an error trying to purge messages in this channel!', ephemeral: true });
        });

        await interaction.reply({ content: `Successfully deleted ${amount} messages.`, ephemeral: true });
    }
};