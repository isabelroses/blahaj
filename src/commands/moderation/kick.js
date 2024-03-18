const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('kick')
        .setDescription('Kicks a user')
        .setDefaultMemberPermissions(PermissionsBitField.Flags.KickMembers)
        .addUserOption(option => option.setName('target').setDescription('The user to kick').setRequired(true))
        .addStringOption(option => option.setName('reason').setDescription('The reason for the kick').setRequired(false)),
    async execute(interaction) {
        const user = interaction.options.getUser('target');
        const member = await interaction.guild.members.fetch(user.id).catch(console.error);
        let reason = interaction.options.getString('reason');
        if (!reason) reason = 'No reason provided';

        if (!interaction.member.permissions.has(PermissionsBitField.Flags.KickMembers)) return await interaction.reply({ content: 'You do not have permission to kick this user', ephemeral: true })
        if (!member.kickable) return await interaction.reply({ content: 'This user cannot be kicked', ephemeral: true })
        if (!member) return await interaction.reply({ content: `User ${user.tag} is not in this server`, ephemeral: true })
        if (interaction.member.id === user.id) return await interaction.reply({ content: 'You cannot kick yourself', ephemeral: true })
        if (member.permissions.has(PermissionsBitField.Flags.Administrator)) return await interaction.reply({ content: 'You cannot kick this user', ephemeral: true })

        user.send(`You have been kicked from ${interaction.guild.name} for ${reason}`).catch(console.error);
        await member.kick(`You have been kicked from ${interaction.guild.name} for ${reason}`).catch(console.log("Dm's are disabled for this user"));
        await interaction.reply({
            content: `Kicked ${user.tag} for ${reason}`,
            ephemeral: true
        })
    }
};
