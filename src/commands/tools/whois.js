const { SlashCommandBuilder, EmbedBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('whois')
        .setDescription('Gets information about a user')
        .addUserOption(option => option.setName('user').setDescription('The user to get information about').setRequired(false)),
    async execute(interaction) {
        const user = interaction.options.getUser('user') || interaction.user;
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
                iconURL: interaction.client.user.displayAvatarURL({ dynamic: true }),
                text: interaction.client.user.tag
            })
            .setTimestamp(Date.now())
            .setAuthor({
                name: interaction.user.tag,
                iconURL: interaction.user.displayAvatarURL({ dynamic: true })
            });
        await interaction.reply({
            embeds: [embed]
        })
    }
};
