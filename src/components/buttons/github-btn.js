module.exports = {
    data: {
        name: 'github',
        description: 'Github button',
        type: 2
    },
    async execute(interaction) {
        await interaction.reply({
            content: 'https://github.com/isabelroses',
        });
    }
}
