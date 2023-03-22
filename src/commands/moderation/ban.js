const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('ban')
        .setDescription('Bans a user')
        .setDefaultMemberPermissions(PermissionsBitField.Flags.BanMembers)
        .addUserOption(option => option.setName('target').setDescription('The user to ban').setRequired(true))
        .addStringOption(option => option.setName('reason').setDescription('The reason for the ban').setRequired(false)),
    async execute(interaction) {
        const user = interaction.options.getUser('target');
        const member = await interaction.guild.members.fetch(user.id).catch(console.error);
        let reason = interaction.options.getString('reason');
        if (!reason) reason = 'No reason provided';

        if (!interaction.member.permissions.has(PermissionsBitField.Flags.BanMembers)) return await interaction.reply({ content: 'You do not have permission to ban this user', ephemeral: true })
        if (!member.kickable) return await interaction.reply({ content: 'This user cannot be banned', ephemeral: true })
        if (!member) return await interaction.reply({ content: `User ${user.tag} is not in this server`, ephemeral: true })
        if (interaction.member.id === user.id) return await interaction.reply({ content: 'You cannot ban yourself', ephemeral: true })
        if (member.permissions.has(PermissionsBitField.Flags.Administrator)) return await interaction.reply({ content: 'You cannot ban this user', ephemeral: true })

        user.send(`You have been banned from ${interaction.guild.name} for ${reason}`).catch(console.log("Dm's are disabled for this user"));
        await member.ban({
            deleteMessageSeconds: 60 * 60 * 24 * 7,
            reason: reason,
        }).catch(console.error);
        await interaction.reply({
            content: `Banned ${user.tag} for ${reason}`,
            ephemeral: true
        })
    }
};
