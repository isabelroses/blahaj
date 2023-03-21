const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('kick')
        .setDescription('Kicks a user')
        .setDefaultPermission(PermissionsBitField.Flags.KickMembers)
        .addUserOption(option => option.setName('user').setDescription('The user to kick').setRequired(true))
        .addStringOption(option => option.setName('reason').setDescription('The reason for the kick').setRequired(false)),
    async execute(interaction, client) {
        const user = interaction.options.getUser('user');
        if (interaction.options.getUser('user').id === interaction.user.id) {
            await interaction.reply({
                content: 'You cannot kick yourself',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.guild.me.id) {
            await interaction.reply({
                content: 'You cannot kick me',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.guild.ownerId) {
            await interaction.reply({
                content: 'You cannot kick the server owner',
                ephemeral: true
            });
        }
        const reason = interaction.options.getString('reason') || 'No reason provided';
        await interaction.guild.members.kick(user, { reason: reason });
        await interaction.reply({
            content: `Kicked ${user.tag} for ${reason}`,
            ephemeral: true
        })
    }
};
