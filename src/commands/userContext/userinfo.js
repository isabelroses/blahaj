const { ContextMenuCommandBuilder, ApplicationCommandType, EmbedBuilder } = require('discord.js');

module.exports = {
    data: new ContextMenuCommandBuilder()
        .setName('User Info')
        .setType(ApplicationCommandType.User),
    async execute(interaction) {
        const user = interaction.options.getUser('user');
        const member = await interaction.guild.members.fetch(user.id);
        const embed = new EmbedBuilder()
            .setTitle(`${user.username}#${user.discriminator}`)
            .setDescription(`ID: ${user.id}`)
            .setColor([255, 255, 255])
            .setThumbnail(user.displayAvatarURL({ dynamic: true }))
            .addFields({ name: 'Created At', value: `<t:${parseInt(user.createdAt / 1000)}:R>`, inline: false })
            .addFields({ name: 'Joined At', value: `<t:${parseInt(member.joinedAt / 1000)}:R>`, inline: true })
            .addFields({ name: 'Bot', value: `${user.bot}`, inline: false })
            .addFields({ name: 'Roles', value: `${member.roles.cache.map(r => r).join(' ')}`, inline: false })
            .setFooter({
                iconURL: user.displayAvatarURL({ dynamic: true }),
                text: user.tag
            })
            .setTimestamp(Date.now())
            .setAuthor({
                name: user.tag,
                iconURL: user.displayAvatarURL({ dynamic: true })
            });
        await interaction.reply({
            embeds: [embed]
        });
    }
};