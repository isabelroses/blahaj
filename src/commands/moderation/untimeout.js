const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('untimeout')
        .setDescription('Untimes out a user')
        .setDefaultPermission(PermissionsBitField.Flags.ModerateMembers)
        .addUserOption(option => option.setName('user').setDescription('The user to untimeout').setRequired(true)),
    async execute(interaction) {
        const user = interaction.options.getUser('user');
        const member = await interaction.guild.members.fetch(user.id).catch(console.error);
        await member.untimeout(time, reason).catch(console.error);
        await interaction.reply({
            content: `Untimed out ${user.tag}`,
            ephemeral: true
        })
    }
};
