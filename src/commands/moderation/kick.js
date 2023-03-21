const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('kick')
        .setDescription('Kicks a user')
        .setDefaultPermission(PermissionsBitField.Flags.KickMembers)
        .addUserOption(option => option.setName('target').setDescription('The user to kick').setRequired(true))
        .addStringOption(option => option.setName('reason').setDescription('The reason for the kick').setRequired(false)),
    async execute(interaction, client) {
        const user = interaction.options.getUser('target');
        const member = await interaction.guild.members.fetch(target.id).catch(console.error);
        const reason = interaction.options.getString('reason') || 'No reason provided';

        user.send(`You have been kicked from ${interaction.guild.name} for ${reason}`).catch(console.error);
        await member.kick(reason);
        await interaction.reply({
            content: `Kicked ${target.tag} for ${reason}`,
            ephemeral: true
        })
    }
};
