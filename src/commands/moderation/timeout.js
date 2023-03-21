const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('timeout')
        .setDescription('Times out a user')
        .setDefaultPermission(PermissionsBitField.Flags.ModerateMembers)
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
        const time = interaction.options.getString('time');
        const reason = interaction.options.getString('reason') || 'No reason provided';
        const member = await interaction.guild.members.fetch(user.id).catch(console.error);
        await member.timeout(time * 1000, reason).catch(console.error);
        await interaction.reply({
            content: `Timed out ${user.tag} for ${time} seconds for ${reason}`,
            ephemeral: true
        })
    }
};
