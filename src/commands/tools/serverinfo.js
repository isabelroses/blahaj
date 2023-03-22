const { SlashCommandBuilder, EmbedBuilder } = require('@discordjs/builders');

module.exports = {
    data: new SlashCommandBuilder()
        .setName('serverinfo')
        .setDescription('Replies with server info!'),
    async execute(interaction) {

        const { guild } = interaction;
        const { members } = guild;
        const { name, ownerId, createdAt, region, memberCount } = guild;
        const icon = guild.iconURL({ dynamic: true });
        const { roles } = guild.roles.cache.size;
        const { emojies } = guild.emojis.cache.size;
        const id = guild.id;

        let baseVerificationLevel = guild.verificationLevel;
        let VerificationLevel;

        if (baseVerificationLevel === '0') VerificationLevel = 'None';
        if (baseVerificationLevel === '1') VerificationLevel = 'Low';
        if (baseVerificationLevel === '2') VerificationLevel = 'Medium';
        if (baseVerificationLevel === '3') VerificationLevel = 'High';
        if (baseVerificationLevel === '4') VerificationLevel = 'Very High';

        const embed = new EmbedBuilder()
            .setTitle('Server Info')
            .setThumbnail(icon)
            .addFields({ name: 'Server Name', value: `${name}`, inline: false })
            .addFields({ name: 'Server ID', value: `${id}`, inline: false })
            .addFields({ name: 'Owner', value: `<@${ownerId}>`, inline: false })
            .addFields({ name: 'Region', value: `${region}`, inline: false })
            .addFields({ name: 'Created At', value: `<t:${parseInt(createdAt / 1000)}:R>`, inline: false })
            .addFields({ name: 'Member Count', value: `${memberCount}`, inline: false })
            .addFields({ name: 'Verification Level', value: `${VerificationLevel}`, inline: false })
            .addFields({ name: 'Roles', value: `${roles}`, inline: false })
            .addFields({ name: 'Emojis', value: `${emojies}`, inline: false })
            .addFields({ name: 'Server Boosts', value: `${guild.premiumSubscriptionCount}`, inline: false })
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
