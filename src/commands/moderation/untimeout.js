const { SlashCommandBuilder, PermissionsBitField } = require('discord.js');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('untimeout')
        .setDescription('Untimes out a user')
        .addUserOption(option => option.setName('user').setDescription('The user to untimeout').setRequired(true)),
    async execute(interaction) {
        if (!interaction.member.permissions.has(PermissionsBitField.FLAGS.TIMEOUT_MEMBERS)) {
            await interaction.reply({
                content: 'You do not have permission to use this command',
                ephemeral: true
            });
        }
        if (!interaction.guild.me.permissions.has(PermissionsBitField.FLAGS.TIMEOUT_MEMBERS)) {
            await interaction.reply({
                content: 'I do not have permission to use this command',
                ephemeral: true
            });
        }
        const user = interaction.options.getUser('user');
        await user.untimeout();
        await interaction.reply({
            content: `Untimed out ${user.tag}`,
            ephemeral: true
        })
    }
};
