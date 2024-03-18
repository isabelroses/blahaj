const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('unban')
        .setDescription('Unbans a user')
        .setDefaultMemberPermissions(PermissionsBitField.Flags.BanMembers)
        .addUserOption(option => option.setName('target').setDescription('The user to unban').setRequired(true)),
    async execute(interaction, client) {
        const userid = interaction.options.getUser('target');

        if (!interaction.member.permissions.has(PermissionsBitField.Flags.BanMembers)) return await interaction.reply({ content: 'You do not have permission to unbanned this user', ephemeral: true })
        if (!member.kickable) return await interaction.reply({ content: 'This user cannot be unbanned out', ephemeral: true })
        if (!member) return await interaction.reply({ content: `User ${user.tag} is not in this server`, ephemeral: true })
        if (interaction.member.id === userid) return await interaction.reply({ content: 'You cannot unbanned yourself', ephemeral: true })
        if (member.permissions.has(PermissionsBitField.Flags.Administrator)) return await interaction.reply({ content: 'You cannot unbanned this user', ephemeral: true })

        await interactions.guild.ban.fetch().then(async bans => {
            if (bans.size == 0) return await interaction.reply({ content: 'There are no banned users in this server', ephemeral: true });
            let bUser = bans.find(b => b.user.id == userid);
            if (!bUser) return await interaction.reply({ content: 'This user is not banned', ephemeral: true });

            await interaction.guild.bans.remove(userid).catch(err => {
                return interaction.reply({ content: 'There was an error unbanning this user', ephemeral: true });
            });
        });
    }
};
