const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('timeout')
        .setDescription('Times out a user')
        .addUserOption(option => option.setName('user').setDescription('The user to time out').setRequired(true))
        .addStringOption(option => option.setName('time').setDescription('The duration to time the user out').setRequired(true).addChoices(
            { name: '60 seconds', value: '60' },
            { name: '5 minutes', value: '300' },
            { name: '10 minutes', value: '600' },
            { name: '30 minutes', value: '1800' },
            { name: '1 hour', value: '3600' },
            { name: '12 hours', value: '43200' },
            { name: '1 day', value: '86400' },
            { name: '1 week', value: '604800' },
            { name: '1 month', value: '2629743' }
        ))
        .addStringOption(option => option.setName('reason').setDescription('The reason for the timeout').setRequired(false)),
    async execute(interaction, client) {
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
        if (interaction.options.getUser('user').id === interaction.user.id) {
            await interaction.reply({
                content: 'You cannot timeout yourself',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.guild.me.id) {
            await interaction.reply({
                content: 'You cannot timeout me',
                ephemeral: true
            });
        }
        if (interaction.options.getUser('user').id === interaction.guild.ownerId) {
            await interaction.reply({
                content: 'You cannot timeout the server owner',
                ephemeral: true
            });
        }
        const reason = interaction.options.getString('reason') || 'No reason provided';
        const duration = interaction.options.getString('time');
        await user.timeout(duration * 1000, reason);
        await interaction.reply({
            content: `Timed out ${user.tag} for ${reason}`,
            ephemeral: true
        })
    }
};
