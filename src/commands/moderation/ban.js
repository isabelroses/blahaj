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
