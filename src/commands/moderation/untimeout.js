const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('untimeout')
        .setDescription('Untimes out a user')
        .setDefaultPermission(PermissionsBitField.Flags.ModerateMembers)
        .addUserOption(option => option.setName('user').setDescription('The user to untimeout').setRequired(true)),
    async execute(interaction) {
        const user = interaction.options.getUser('user');
        await user.untimeout();
        await interaction.reply({
            content: `Untimed out ${user.tag}`,
            ephemeral: true
        })
    }
};
