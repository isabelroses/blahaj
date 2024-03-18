const { SlashCommandBuilder, EmbedBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('serverinfo')
        .setDescription('Replies with server info!'),
    async execute(interaction) {

        const { guild } = interaction;
        const { name, ownerId, createdAt, memberCount } = guild;
        const icon = guild.iconURL({ dynamic: true });
        const roles = guild.roles.cache.size;
        const emojies = guild.emojis.cache.size;
        const id = guild.id;

        let baseVerification = guild.verificationLevel;
        let verificationLevel = '';

        if (baseVerification == 0) verificationLevel = 'None';
        if (baseVerification == 1) verificationLevel = 'Low';
        if (baseVerification == 2) verificationLevel = 'Medium';
        if (baseVerification == 3) verificationLevel = 'High';
        if (baseVerification == 4) verificationLevel = 'Very High';

        const embed = new EmbedBuilder()
            .setTitle('Server Info')
            .setThumbnail(icon)
            .addFields({ name: 'Server Name', value: `${name}`, inline: false })
            .addFields({ name: 'Server ID', value: `${id}`, inline: false })
            .addFields({ name: 'Owner', value: `<@${ownerId}>`, inline: false })
            .addFields({ name: 'Created At', value: `<t:${parseInt(createdAt / 1000)}:R>`, inline: false })
            .addFields({ name: 'Member Count', value: `${memberCount}`, inline: false })
            .addFields({ name: 'Verification Level', value: `${verificationLevel}`, inline: false })
            .addFields({ name: 'Roles', value: `${roles}`, inline: true })
            .addFields({ name: 'Emojis', value: `${emojies}`, inline: true })
            .addFields({ name: 'Server Boosts', value: `${guild.premiumSubscriptionCount}`, inline: true })
            .setColor([255, 255, 255])
            .setFooter({
                text: interaction.user.tag,
                iconURL: interaction.user.displayAvatarURL({ dynamic: true })
            })
            .setTimestamp(Date.now())
            .setAuthor({ name: name, iconURL: icon });

        await interaction.reply({ embeds: [embed] });
    },
};
