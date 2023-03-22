const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('timeout')
        .setDescription('Times out a user')
        .setDefaultMemberPermissions(PermissionsBitField.Flags.ModerateMembers)
        .addUserOption(option => option.setName('target').setDescription('The user to time out').setRequired(true))
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
    async execute(interaction) {
        const user = interaction.options.getUser('target');
        const member = await interaction.guild.members.fetch(user.id).catch(console.error);
        let reason = interaction.options.getString('reason');
        let time = interaction.options.getString('time');
        if (!time) time = '60';
        if (!reason) reason = 'No reason provided';

        if (!interaction.member.permissions.has(PermissionsBitField.Flags.ModerateMembers)) return await interaction.reply({ content: 'You do not have permission to timeout this user', ephemeral: true })
        if (!member.kickable) return await interaction.reply({ content: 'This user cannot be timed out', ephemeral: true })
        if (!member) return await interaction.reply({ content: `User ${user.tag} is not in this server`, ephemeral: true })
        if (interaction.member.id === user.id) return await interaction.reply({ content: 'You cannot timeout yourself', ephemeral: true })
        if (member.permissions.has(PermissionsBitField.Flags.Administrator)) return await interaction.reply({ content: 'You cannot timeout this user', ephemeral: true })

        await member.timeout(time * 1000, reason).catch(console.error);
        await interaction.reply({
            content: `Timed out ${user.tag} for ${time} seconds for ${reason}`,
            ephemeral: true
        })
    }
};
