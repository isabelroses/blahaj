const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('ban')
        .setDescription('Bans a user')
        .addUserOption(option => option.setName('user').setDescription('The user to ban').setRequired(true))
        .addStringOption(option => option.setName('reason').setDescription('The reason for the ban').setRequired(false)),
    async execute(interaction) {
        const user = interaction.options.getUser('user');
        if (!interaction.guild.members.fetch(user.id).permissions.has(PermissionsBitField.Flags.BanMembers)) {
            await interaction.reply({
                content: 'You do not have permission to use this command',
                ephemeral: true
            });
        }
        if (!interaction.guild.members.me.permissions.has(PermissionsBitField.Flags.BanMembers)) {
            await interaction.reply({
                content: 'I do not have permission to use this command',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.user.id) {
            await interaction.reply({
                content: 'You cannot ban yourself',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.guild.me.id) {
            await interaction.reply({
                content: 'You cannot ban me',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.guild.ownerId) {
            await interaction.reply({
                content: 'You cannot ban the server owner',
                ephemeral: true
            });
        }
        const reason = interaction.options.getString('reason') || 'No reason provided';
        await interaction.guild.bans.create(user.id, { reason });
        await interaction.reply({
            content: `Banned ${user.tag} for ${reason}`,
            ephemeral: true
        })
    }
};
