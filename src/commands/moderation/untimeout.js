const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('untimeout')
        .setDescription('Untimes out a user')
        .setDefaultMemberPermissions(PermissionsBitField.Flags.ModerateMembers)
        .addUserOption(option => option.setName('target').setDescription('The user to untimeout').setRequired(true)),
    async execute(interaction) {
        const user = interaction.options.getUser('target');
        const member = await interaction.guild.members.fetch(user.id).catch(console.error);

        if (!interaction.member.permissions.has(PermissionsBitField.Flags.ModerateMembers)) return await interaction.reply({ content: 'You do not have permission to untimeout this user', ephemeral: true })
        if (!member.isTimeout) return await interaction.reply({ content: `User ${user.tag} is not timed out`, ephemeral: true })
        if (!member) return await interaction.reply({ content: `User ${user.tag} is not in this server`, ephemeral: true })
        if (interaction.member.id === user.id) return await interaction.reply({ content: 'You cannot untimeout yourself', ephemeral: true })
        if (member.permissions.has(PermissionsBitField.Flags.Administrator)) return await interaction.reply({ content: 'You cannot untimeout this user', ephemeral: true })

        await member.timeout(null).catch(console.error);
        await interaction.reply({
            content: `Untimed out ${user.tag}`,
            ephemeral: true
        })
    }
};
