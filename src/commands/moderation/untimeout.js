const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('untimeout')
        .setDescription('Untimes out a user')
        .addUserOption(option => option.setName('user').setDescription('The user to untimeout').setRequired(true)),
    async execute(interaction) {
        const user = interaction.options.getUser('user');
        if (!interaction.guild.members.fetch(user.id).permissions.has(PermissionsBitField.Flags.ModerateMembers)) {
            await interaction.reply({
                content: 'You do not have permission to use this command',
                ephemeral: true
            });
        }
        if (!interaction.guild.members.me.permissions.has(PermissionsBitField.Flags.ModerateMembers)) {
            await interaction.reply({
                content: 'I do not have permission to use this command',
                ephemeral: true
            });
        }
        await user.untimeout();
        await interaction.reply({
            content: `Untimed out ${user.tag}`,
            ephemeral: true
        })
    }
};
