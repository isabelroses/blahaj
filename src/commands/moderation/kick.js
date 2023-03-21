const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('kick')
        .setDescription('Kicks a user')
        .addUserOption(option => option.setName('user').setDescription('The user to kick').setRequired(true))
        .addStringOption(option => option.setName('reason').setDescription('The reason for the kick').setRequired(false)),
    async execute(interaction) {
        if (!interaction.member.permissions.has(PermissionsBitField.Flags.KickMembers)) {
            await interaction.reply({
                content: 'You do not have permission to use this command',
                ephemeral: true
            });
        }
        if (!interaction.guild.me.permissions.has(PermissionsBitField.Flags.KickMembers)) {
            await interaction.reply({
                content: 'I do not have permission to use this command',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.user.id) {
            await interaction.reply({
                content: 'You cannot kick yourself',
                ephemeral: true
            });
        }
        const user = interaction.options.getUser('user');
        const reason = interaction.options.getString('reason') || 'No reason provided';
        await interaction.guild.members.kick(user, { reason: reason });
        await interaction.reply({
            content: `Kicked ${user.tag} for ${reason}`,
            ephemeral: true
        })
    }
};
